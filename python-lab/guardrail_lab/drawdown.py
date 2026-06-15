"""Drawdown analytics over a NAV curve.

Standard-library only. Functions are pure: they take a NAV curve (a list of
``(timestamp, nav)`` points) and return dataclasses / dicts. The NAV curve is
typically produced by :func:`guardrail_lab.metrics.nav_series` from the event
log, so this module composes with the existing loaders rather than
re-implementing NAV extraction.

Terminology:

* **Underwater series** — at each point, the percentage decline from the
  running peak NAV (``0.0`` at a fresh high, negative below it).
* **Drawdown episode** — a contiguous span that begins when NAV falls below a
  peak and ends when NAV recovers back to (or above) that peak. The trough is
  the lowest NAV within the span.
* **Recovery time** — wall-clock time from the trough back to the prior peak.
  An episode still underwater at the end of the curve has no recovery time.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone

from .metrics import nav_series

NavPoint = tuple[str, float]


@dataclass(frozen=True)
class UnderwaterPoint:
    """A single point on the underwater curve.

    Attributes:
        timestamp: The NAV point's timestamp (verbatim from the source).
        nav: The NAV value at this point.
        peak: The running peak NAV up to and including this point.
        drawdown_pct: Percentage decline from ``peak`` (``<= 0.0``).
    """

    timestamp: str
    nav: float
    peak: float
    drawdown_pct: float


@dataclass(frozen=True)
class DrawdownEpisode:
    """A single peak-to-trough-to-recovery drawdown episode.

    Attributes:
        peak_timestamp: Timestamp of the peak the decline started from.
        peak_nav: NAV at that peak.
        trough_timestamp: Timestamp of the lowest NAV in the episode.
        trough_nav: NAV at the trough.
        depth_pct: Maximum decline within the episode as a negative
            percentage (e.g. ``-4.32`` for a 4.32% drawdown).
        drawdown_seconds: Wall-clock seconds from peak to trough, or ``None``
            when timestamps cannot be parsed.
        recovery_timestamp: Timestamp at which NAV first reached the prior peak
            again, or ``None`` if it never recovered within the curve.
        recovery_seconds: Wall-clock seconds from trough to recovery, or
            ``None`` if unrecovered / timestamps unparseable.
        recovered: Whether NAV climbed back to the prior peak before the curve
            ended.
    """

    peak_timestamp: str
    peak_nav: float
    trough_timestamp: str
    trough_nav: float
    depth_pct: float
    drawdown_seconds: float | None
    recovery_timestamp: str | None
    recovery_seconds: float | None
    recovered: bool


@dataclass(frozen=True)
class DrawdownReport:
    """Summary of drawdown behaviour over a NAV curve.

    Attributes:
        points: The full underwater series.
        max_drawdown_pct: The single worst drawdown percentage observed
            (``0.0`` when the curve never declines or is empty).
        peak_timestamp: Timestamp of the peak preceding the worst drawdown,
            or ``None``.
        trough_timestamp: Timestamp of the worst-drawdown trough, or ``None``.
        max_drawdown_seconds: Peak-to-trough duration of the worst drawdown,
            or ``None``.
        max_recovery_seconds: Trough-to-recovery duration of the worst
            drawdown, or ``None`` if it never recovered.
        episodes: All drawdown episodes, deepest first.
    """

    points: list[UnderwaterPoint] = field(default_factory=list)
    max_drawdown_pct: float = 0.0
    peak_timestamp: str | None = None
    trough_timestamp: str | None = None
    max_drawdown_seconds: float | None = None
    max_recovery_seconds: float | None = None
    episodes: list[DrawdownEpisode] = field(default_factory=list)


def _parse_timestamp(value: object) -> datetime | None:
    """Parse a timestamp string/number into a timezone-aware ``datetime``.

    Accepts ISO-8601 (optional trailing ``Z``) and numeric epochs (seconds or
    milliseconds). Returns ``None`` when uninterpretable.
    """
    if value is None:
        return None

    if isinstance(value, (int, float)):
        epoch = float(value)
        if epoch > 1e11:
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


def _seconds_between(start: str, end: str) -> float | None:
    """Wall-clock seconds between two timestamps, or ``None`` if unparseable."""
    a = _parse_timestamp(start)
    b = _parse_timestamp(end)
    if a is None or b is None:
        return None
    return (b - a).total_seconds()


def underwater_series(nav_curve: list[NavPoint]) -> list[UnderwaterPoint]:
    """Compute the underwater (drawdown-from-peak) series.

    Args:
        nav_curve: Ordered ``(timestamp, nav)`` points.

    Returns:
        One :class:`UnderwaterPoint` per input point. Peaks at or below zero
        report ``0.0`` drawdown to avoid division by zero. The input is not
        mutated.
    """
    result: list[UnderwaterPoint] = []
    peak = float("-inf")
    for timestamp, nav in nav_curve:
        peak = max(peak, nav)
        if peak > 0:
            drawdown_pct = (nav - peak) / peak * 100.0
        else:
            drawdown_pct = 0.0
        result.append(
            UnderwaterPoint(
                timestamp=timestamp,
                nav=nav,
                peak=peak,
                drawdown_pct=round(drawdown_pct, 6),
            )
        )
    return result


def drawdown_episodes(nav_curve: list[NavPoint]) -> list[DrawdownEpisode]:
    """Identify every peak-to-trough-to-recovery drawdown episode.

    An episode opens when NAV first dips below a running peak and closes when
    NAV climbs back to that peak (recovery) — or remains open through the end
    of the curve (unrecovered). The deepest point within the span is the
    trough.

    Args:
        nav_curve: Ordered ``(timestamp, nav)`` points.

    Returns:
        Episodes sorted by depth (deepest / most-negative ``depth_pct`` first).
    """
    episodes: list[DrawdownEpisode] = []

    peak_ts: str | None = None
    peak_nav = float("-inf")
    in_episode = False
    trough_ts: str | None = None
    trough_nav = float("inf")

    def close(recovery_ts: str | None) -> None:
        """Emit the in-progress episode and reset trough tracking."""
        if peak_ts is None or trough_ts is None or peak_nav <= 0:
            return
        depth_pct = (trough_nav - peak_nav) / peak_nav * 100.0
        dd_seconds = _seconds_between(peak_ts, trough_ts)
        if recovery_ts is not None:
            rec_seconds = _seconds_between(trough_ts, recovery_ts)
        else:
            rec_seconds = None
        episodes.append(
            DrawdownEpisode(
                peak_timestamp=peak_ts,
                peak_nav=peak_nav,
                trough_timestamp=trough_ts,
                trough_nav=trough_nav,
                depth_pct=round(depth_pct, 6),
                drawdown_seconds=dd_seconds,
                recovery_timestamp=recovery_ts,
                recovery_seconds=rec_seconds,
                recovered=recovery_ts is not None,
            )
        )

    for timestamp, nav in nav_curve:
        if nav >= peak_nav:
            # New high: close any open episode (recovered at this point) then
            # advance the peak.
            if in_episode:
                close(recovery_ts=timestamp)
                in_episode = False
                trough_nav = float("inf")
                trough_ts = None
            peak_nav = nav
            peak_ts = timestamp
        else:
            # Below the peak: we are (or now become) underwater.
            in_episode = True
            if nav < trough_nav:
                trough_nav = nav
                trough_ts = timestamp

    if in_episode:
        close(recovery_ts=None)

    episodes.sort(key=lambda ep: ep.depth_pct)
    return episodes


def worst_episodes(
    nav_curve: list[NavPoint], top_n: int = 5
) -> list[DrawdownEpisode]:
    """Return the ``top_n`` deepest drawdown episodes.

    Args:
        nav_curve: Ordered ``(timestamp, nav)`` points.
        top_n: Maximum number of episodes to return (``<= 0`` returns none).

    Returns:
        The deepest episodes, deepest first.
    """
    if top_n <= 0:
        return []
    return drawdown_episodes(nav_curve)[:top_n]


def analyze_drawdown(
    nav_curve: list[NavPoint], top_n: int = 5
) -> DrawdownReport:
    """Compute a full drawdown report from a NAV curve.

    Args:
        nav_curve: Ordered ``(timestamp, nav)`` points.
        top_n: How many worst episodes to retain in the report.

    Returns:
        A :class:`DrawdownReport`. An empty or non-declining curve yields a
        report with ``max_drawdown_pct == 0.0`` and no episodes.
    """
    points = underwater_series(nav_curve)
    episodes = drawdown_episodes(nav_curve)

    if episodes:
        worst = episodes[0]  # already sorted deepest-first
        return DrawdownReport(
            points=points,
            max_drawdown_pct=worst.depth_pct,
            peak_timestamp=worst.peak_timestamp,
            trough_timestamp=worst.trough_timestamp,
            max_drawdown_seconds=worst.drawdown_seconds,
            max_recovery_seconds=worst.recovery_seconds,
            episodes=episodes[:top_n] if top_n > 0 else [],
        )

    return DrawdownReport(points=points)


def analyze_drawdown_from_events(
    events: list[dict], top_n: int = 5
) -> DrawdownReport:
    """Convenience wrapper: build the NAV curve from events, then analyze.

    Reuses :func:`guardrail_lab.metrics.nav_series` to extract the NAV curve
    from ``portfolio_reconciled`` events before delegating to
    :func:`analyze_drawdown`.

    Args:
        events: Parsed event log (e.g. from
            :func:`guardrail_lab.db.load_events`).
        top_n: How many worst episodes to retain.

    Returns:
        A :class:`DrawdownReport`.
    """
    return analyze_drawdown(nav_series(events), top_n=top_n)
