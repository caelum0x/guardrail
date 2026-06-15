"""Regime analytics derived from the Guardrail Alpha event log.

Standard-library only. All functions are pure: they take the parsed event
list (as produced by :func:`guardrail_lab.db.load_events`) and return
dataclasses / dicts without touching the filesystem or mutating their inputs.

The agent emits, per cycle, in timestamp order::

    regime_classified  -> {"regime": "risk_on"}
    asset_scored       -> {"symbol": "SHIB", "score": 0.718}
    order_proposed     -> {"from": "USDT", "to": "SHIB", "amount_usd": "1700.00"}
    risk_approved | risk_clipped -> {"amount_usd": "<final>"}
    tx_confirmed       -> {...}

This module computes three things over that stream:

* a **regime-transition matrix** (raw counts plus row-normalized
  probabilities of moving from one regime to the next),
* the **time-in-regime distribution** (how many classifications, and how much
  wall-clock time, were spent in each regime), and
* the **average exposure multiplier per regime** — the mean proposed order
  size while a regime was active, expressed as a multiple of the overall mean
  proposed order size, so ``1.0`` means "typical sizing" and ``> 1.0`` means
  the agent leaned in during that regime.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone

REGIME_EVENT = "regime_classified"
ORDER_EVENT = "order_proposed"
UNKNOWN_REGIME = "unknown"


@dataclass(frozen=True)
class RegimeTransitionMatrix:
    """Regime-to-regime transition counts and probabilities.

    Attributes:
        regimes: Sorted list of every regime label observed.
        counts: ``counts[a][b]`` is the number of times the agent moved
            directly from regime ``a`` to regime ``b`` (consecutive
            classifications). Self-transitions (``a == b``) are included.
        probabilities: Row-normalized version of ``counts``. Each
            ``probabilities[a]`` sums to ``1.0`` (or is all-zero when regime
            ``a`` never had a successor). Values are fractions in ``[0, 1]``.
        total_transitions: Total number of consecutive regime pairs observed.
    """

    regimes: list[str]
    counts: dict[str, dict[str, int]]
    probabilities: dict[str, dict[str, float]]
    total_transitions: int


@dataclass(frozen=True)
class TimeInRegime:
    """How long the agent spent in a single regime.

    Attributes:
        regime: The regime label.
        classifications: Number of ``regime_classified`` events for it.
        seconds: Total wall-clock seconds attributed to the regime (the gap
            between each classification and the next one, with the final
            classification contributing ``0.0`` because it has no successor).
            ``0.0`` when timestamps cannot be parsed.
        fraction: ``classifications`` divided by the total number of
            classifications, in ``[0, 1]``.
    """

    regime: str
    classifications: int
    seconds: float
    fraction: float


@dataclass(frozen=True)
class RegimeExposure:
    """Average proposed-order exposure while a regime was active.

    Attributes:
        regime: The regime label.
        order_count: Number of ``order_proposed`` events seen while the
            regime was active.
        avg_order_usd: Mean proposed order size (USD) during the regime;
            ``0.0`` when no orders were proposed.
        exposure_multiplier: ``avg_order_usd`` divided by the overall mean
            proposed order size across all regimes. ``1.0`` is baseline sizing;
            ``> 1.0`` means larger-than-average orders during this regime.
            ``0.0`` when there is no baseline (no orders anywhere).
    """

    regime: str
    order_count: int
    avg_order_usd: float
    exposure_multiplier: float


@dataclass(frozen=True)
class RegimeAnalysis:
    """Bundle of every regime analytic computed from an event log."""

    transitions: RegimeTransitionMatrix
    time_in_regime: list[TimeInRegime] = field(default_factory=list)
    exposure: list[RegimeExposure] = field(default_factory=list)


def _payload(event: dict) -> dict:
    """Return an event payload as a dict, never ``None``."""
    payload = event.get("payload")
    return payload if isinstance(payload, dict) else {}


def _regime_label(event: dict) -> str:
    """Extract a clean regime label from a ``regime_classified`` event."""
    raw = _payload(event).get("regime")
    if isinstance(raw, str) and raw.strip():
        return raw.strip()
    return UNKNOWN_REGIME


def _to_float(value: object) -> float | None:
    """Best-effort conversion of a payload value (often a decimal string)."""
    if value is None:
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def _parse_timestamp(value: object) -> datetime | None:
    """Parse an event timestamp into a timezone-aware ``datetime``.

    Accepts ISO-8601 strings (with or without a trailing ``Z``) and numeric
    epoch values (seconds, or milliseconds when the magnitude is large).
    Returns ``None`` when the value cannot be interpreted.
    """
    if value is None:
        return None

    if isinstance(value, (int, float)):
        epoch = float(value)
        if epoch > 1e11:  # almost certainly milliseconds
            epoch /= 1000.0
        try:
            return datetime.fromtimestamp(epoch, tz=timezone.utc)
        except (OverflowError, OSError, ValueError):
            return None

    text = str(value).strip()
    if not text:
        return None

    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    try:
        parsed = datetime.fromisoformat(text)
    except ValueError:
        # Fall back to a bare epoch encoded as a string.
        try:
            epoch = float(str(value))
        except (TypeError, ValueError):
            return None
        if epoch > 1e11:
            epoch /= 1000.0
        try:
            return datetime.fromtimestamp(epoch, tz=timezone.utc)
        except (OverflowError, OSError, ValueError):
            return None

    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed


def _ordered_events(events: list[dict]) -> list[dict]:
    """Return events sorted by ``(timestamp, id)`` without mutating the input."""
    return sorted(
        events,
        key=lambda event: (event.get("timestamp") or "", event.get("id") or 0),
    )


def regime_sequence(events: list[dict]) -> list[tuple[str, str]]:
    """Ordered list of ``(timestamp, regime)`` from ``regime_classified`` events.

    Args:
        events: Parsed event log.

    Returns:
        A list of ``(timestamp, regime)`` tuples in timestamp order. The
        timestamp is whatever string the event carried (possibly empty); the
        regime is normalized to a non-empty label (``"unknown"`` if missing).
    """
    sequence: list[tuple[str, str]] = []
    for event in _ordered_events(events):
        if event.get("event_type") != REGIME_EVENT:
            continue
        sequence.append((event.get("timestamp") or "", _regime_label(event)))
    return sequence


def transition_matrix(events: list[dict]) -> RegimeTransitionMatrix:
    """Compute the regime-to-regime transition matrix.

    Consecutive ``regime_classified`` events form transitions: each adjacent
    pair ``(previous, current)`` increments ``counts[previous][current]``.
    Probabilities are the row-normalized counts.

    Args:
        events: Parsed event log.

    Returns:
        A :class:`RegimeTransitionMatrix`. When fewer than two classifications
        exist the matrix is empty (no transitions) but still lists any single
        regime that was observed.
    """
    sequence = [regime for _, regime in regime_sequence(events)]
    regimes = sorted(set(sequence))

    counts: dict[str, dict[str, int]] = {
        a: {b: 0 for b in regimes} for a in regimes
    }
    total = 0
    for previous, current in zip(sequence, sequence[1:]):
        counts[previous][current] += 1
        total += 1

    probabilities: dict[str, dict[str, float]] = {}
    for a in regimes:
        row_total = sum(counts[a].values())
        if row_total > 0:
            probabilities[a] = {
                b: counts[a][b] / row_total for b in regimes
            }
        else:
            probabilities[a] = {b: 0.0 for b in regimes}

    return RegimeTransitionMatrix(
        regimes=regimes,
        counts=counts,
        probabilities=probabilities,
        total_transitions=total,
    )


def time_in_regime(events: list[dict]) -> list[TimeInRegime]:
    """Compute the time-in-regime distribution.

    Each ``regime_classified`` event is held until the next one; the elapsed
    wall-clock time between them is attributed to the first regime. The final
    classification has no successor and contributes ``0.0`` seconds. The
    ``fraction`` field is always available (count-based) even when timestamps
    are unparseable.

    Args:
        events: Parsed event log.

    Returns:
        A list of :class:`TimeInRegime`, sorted by descending classification
        count then regime name.
    """
    sequence = regime_sequence(events)
    total_classifications = len(sequence)

    counts: dict[str, int] = {}
    seconds: dict[str, float] = {}
    for regime in (r for _, r in sequence):
        counts[regime] = counts.get(regime, 0) + 1
        seconds.setdefault(regime, 0.0)

    for (ts_a, regime_a), (ts_b, _regime_b) in zip(sequence, sequence[1:]):
        start = _parse_timestamp(ts_a)
        end = _parse_timestamp(ts_b)
        if start is None or end is None:
            continue
        delta = (end - start).total_seconds()
        if delta > 0:
            seconds[regime_a] += delta

    result = [
        TimeInRegime(
            regime=regime,
            classifications=count,
            seconds=round(seconds.get(regime, 0.0), 3),
            fraction=(
                count / total_classifications
                if total_classifications
                else 0.0
            ),
        )
        for regime, count in counts.items()
    ]
    result.sort(key=lambda item: (-item.classifications, item.regime))
    return result


def exposure_by_regime(events: list[dict]) -> list[RegimeExposure]:
    """Compute the average exposure multiplier per regime.

    Walks the event log in order, tracking the currently active regime (the
    most recent ``regime_classified``). Every ``order_proposed`` seen while a
    regime is active contributes its ``amount_usd`` to that regime's average.
    The per-regime average is then divided by the overall mean proposed-order
    size to yield an exposure multiplier (``1.0`` == baseline sizing).

    Args:
        events: Parsed event log.

    Returns:
        A list of :class:`RegimeExposure`, sorted by descending exposure
        multiplier then regime name. Orders proposed before any regime
        classification are attributed to ``"unknown"``.
    """
    current_regime = UNKNOWN_REGIME
    order_count: dict[str, int] = {}
    order_total: dict[str, float] = {}

    for event in _ordered_events(events):
        event_type = event.get("event_type")
        if event_type == REGIME_EVENT:
            current_regime = _regime_label(event)
        elif event_type == ORDER_EVENT:
            amount = _to_float(_payload(event).get("amount_usd")) or 0.0
            order_count[current_regime] = order_count.get(current_regime, 0) + 1
            order_total[current_regime] = (
                order_total.get(current_regime, 0.0) + amount
            )

    grand_total = sum(order_total.values())
    grand_count = sum(order_count.values())
    overall_mean = grand_total / grand_count if grand_count else 0.0

    result: list[RegimeExposure] = []
    for regime, count in order_count.items():
        avg = order_total[regime] / count if count else 0.0
        multiplier = avg / overall_mean if overall_mean > 0 else 0.0
        result.append(
            RegimeExposure(
                regime=regime,
                order_count=count,
                avg_order_usd=round(avg, 2),
                exposure_multiplier=round(multiplier, 4),
            )
        )

    result.sort(key=lambda item: (-item.exposure_multiplier, item.regime))
    return result


def analyze_regimes(events: list[dict]) -> RegimeAnalysis:
    """Run every regime analytic over an event log.

    Args:
        events: Parsed event log (e.g. from
            :func:`guardrail_lab.db.load_events`).

    Returns:
        A :class:`RegimeAnalysis` bundling the transition matrix, the
        time-in-regime distribution, and the per-regime exposure multipliers.
    """
    return RegimeAnalysis(
        transitions=transition_matrix(events),
        time_in_regime=time_in_regime(events),
        exposure=exposure_by_regime(events),
    )
