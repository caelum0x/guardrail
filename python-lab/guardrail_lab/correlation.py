"""Correlation and exposure analytics for Guardrail Alpha.

Standard-library only (``json``, ``math``, ``glob``, ``dataclasses``,
``pathlib``). No numpy / pandas.

This module turns the agent's recorded market history into two complementary
risk views:

* a **per-asset return series + pairwise correlation matrix** — derived from the
  snapshot history under ``data/snapshots/<run_id>.jsonl`` when present (each
  snapshot file is one timestamped market observation carrying a per-asset
  ``price_usd``). Reading several snapshots in timestamp order yields one price
  point per asset per snapshot; consecutive prices give simple returns, and the
  pairwise **Pearson** correlation is computed over the overlapping return
  observations of each asset pair. When no snapshot history is available the
  module falls back to the event log (``market_snapshot_received`` events), which
  in the current schema carry only counts — so the fallback degrades to an empty
  result with a clear reason rather than fabricating prices.

* a **concentration / exposure summary** — per-asset gross weight taken from the
  latest target book (the agent's run report ``positions[].weight_pct``), plus
  the **Herfindahl-Hirschman index** (sum of squared weight fractions) and the
  **effective number of positions** (``1 / HHI``), the standard concentration
  diagnostics.

Design contract (matches the rest of ``guardrail_lab``):

* Pure functions over already-parsed / file-path inputs; inputs are never
  mutated.
* Frozen, type-annotated dataclasses.
* **Never raises on empty / malformed input** — every entry point returns a
  clearly-empty result carrying a human-readable ``reason`` instead.
"""

from __future__ import annotations

import glob
import json
import math
from dataclasses import dataclass, field
from pathlib import Path

DEFAULT_SNAPSHOT_DIR = "data/snapshots"
DEFAULT_REPORT = "data/run_report.json"
DEFAULT_DB = "data/guardrail_alpha.db"

SNAPSHOT_EVENT = "market_snapshot_received"

# A correlation needs at least this many paired return observations to be
# meaningful; below it the pair is reported as having insufficient data.
MIN_PAIRED_RETURNS = 2


@dataclass(frozen=True)
class AssetReturnSeries:
    """Per-asset simple-return series derived from the price history.

    Attributes:
        symbol: Asset ticker (e.g. ``"WBNB"``).
        timestamps: Return timestamps (the *later* timestamp of each
            consecutive price pair), in ascending order.
        returns: Simple returns ``(p_t / p_{t-1}) - 1`` aligned with
            ``timestamps``. Always the same length as ``timestamps``.
        mean: Mean of ``returns`` (``0.0`` when empty).
    """

    symbol: str
    timestamps: tuple[str, ...]
    returns: tuple[float, ...]
    mean: float


@dataclass(frozen=True)
class CorrelationPair:
    """Pearson correlation between two assets over their overlapping returns.

    Attributes:
        a: First asset symbol (lexicographically the smaller of the pair).
        b: Second asset symbol.
        correlation: Pearson correlation coefficient in ``[-1, 1]``; ``0.0``
            when undefined (insufficient overlap or a zero-variance series).
        observations: Number of paired return observations used.
        defined: ``True`` when the coefficient was computable from at least
            :data:`MIN_PAIRED_RETURNS` observations with non-zero variance in
            both legs.
    """

    a: str
    b: str
    correlation: float
    observations: int
    defined: bool


@dataclass(frozen=True)
class CorrelationMatrix:
    """Pairwise Pearson correlation matrix over a set of assets.

    Attributes:
        symbols: Sorted list of asset symbols included in the matrix.
        matrix: ``matrix[a][b]`` is the Pearson correlation between ``a`` and
            ``b`` (``1.0`` on the diagonal, symmetric off-diagonal). Pairs with
            insufficient data carry ``0.0``.
        pairs: Off-diagonal :class:`CorrelationPair` records (one per unordered
            pair), sorted by descending absolute correlation then symbols.
        n_observations: Number of snapshots that contributed price points.
    """

    symbols: list[str]
    matrix: dict[str, dict[str, float]]
    pairs: list[CorrelationPair]
    n_observations: int


@dataclass(frozen=True)
class AssetExposure:
    """Gross weight of a single position in the latest target book.

    Attributes:
        symbol: Asset ticker.
        weight: Gross weight as a fraction of the book in ``[0, 1]``
            (e.g. ``0.12`` for a 12% position).
        weight_pct: The same weight expressed in percent (``weight * 100``).
    """

    symbol: str
    weight: float
    weight_pct: float


@dataclass(frozen=True)
class ExposureSummary:
    """Concentration diagnostics for the latest target book.

    Attributes:
        positions: Per-asset gross weights, sorted by descending weight then
            symbol.
        gross_weight: Sum of the position weights (``~1.0`` for a fully
            invested book; less when cash is held).
        herfindahl: Herfindahl-Hirschman index — the sum of squared weight
            fractions, in ``(0, 1]``. ``1.0`` means everything is in one
            position; lower means more diversified.
        effective_positions: Effective number of positions (``1 / herfindahl``);
            ``0.0`` when there are no positions.
        position_count: Number of distinct positions in the book.
    """

    positions: list[AssetExposure]
    gross_weight: float
    herfindahl: float
    effective_positions: float
    position_count: int


@dataclass(frozen=True)
class CorrelationReport:
    """Bundle of correlation + exposure analytics with an overall status.

    Attributes:
        ok: ``True`` when at least one of the two analytics produced usable
            output (a non-empty correlation matrix or a non-empty exposure
            summary).
        reason: Human-readable explanation when ``ok`` is ``False`` (and a
            short status note otherwise). Never raises — this is how empty
            input is reported.
        source: Where the price/return history was read from
            (``"snapshots"``, ``"event-log"``, or ``"none"``).
        correlation: The :class:`CorrelationMatrix` (possibly empty).
        exposure: The :class:`ExposureSummary` (possibly empty).
        series: The per-asset return series used to build the matrix.
    """

    ok: bool
    reason: str
    source: str
    correlation: CorrelationMatrix
    exposure: ExposureSummary
    series: list[AssetReturnSeries] = field(default_factory=list)


def _empty_matrix(n_observations: int = 0) -> CorrelationMatrix:
    """Return an empty correlation matrix."""
    return CorrelationMatrix(
        symbols=[], matrix={}, pairs=[], n_observations=n_observations
    )


def _empty_exposure() -> ExposureSummary:
    """Return an empty exposure summary."""
    return ExposureSummary(
        positions=[],
        gross_weight=0.0,
        herfindahl=0.0,
        effective_positions=0.0,
        position_count=0,
    )


def _to_float(value: object) -> float | None:
    """Best-effort conversion of a payload value (often a decimal string)."""
    if value is None:
        return None
    try:
        result = float(value)
    except (TypeError, ValueError):
        return None
    if math.isnan(result) or math.isinf(result):
        return None
    return result


def _iter_snapshot_records(snapshot_dir: str) -> list[dict]:
    """Read every snapshot record under ``snapshot_dir`` in timestamp order.

    Each ``*.jsonl`` file may hold one or more JSON objects (one per line). Lines
    that fail to parse, or that are not objects, are skipped rather than raising.
    Records are sorted by ``timestamp_ms`` (missing timestamps sort last).
    """
    directory = Path(snapshot_dir)
    if not directory.is_dir():
        return []

    records: list[dict] = []
    for path in sorted(directory.glob("*.jsonl")):
        try:
            text = path.read_text(encoding="utf-8")
        except OSError:
            continue
        for line in text.splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(obj, dict) and isinstance(obj.get("assets"), list):
                records.append(obj)

    records.sort(key=lambda rec: _to_float(rec.get("timestamp_ms")) or float("inf"))
    return records


def _prices_by_asset(records: list[dict]) -> tuple[list[str], dict[str, list[float | None]]]:
    """Build an aligned price table from ordered snapshot records.

    Returns ``(timestamps, prices)`` where ``timestamps`` has one entry per
    record and ``prices[symbol]`` is a list aligned to ``timestamps`` (``None``
    where that asset was absent or unpriced in a given snapshot).
    """
    timestamps: list[str] = []
    prices: dict[str, list[float | None]] = {}

    for index, record in enumerate(records):
        ts_value = record.get("timestamp_ms")
        timestamps.append(str(ts_value) if ts_value is not None else str(index))

    for index, record in enumerate(records):
        seen: set[str] = set()
        for entry in record.get("assets", []):
            if not isinstance(entry, dict):
                continue
            asset = entry.get("asset")
            symbol = None
            if isinstance(asset, dict):
                symbol = asset.get("symbol")
            if not isinstance(symbol, str) or not symbol.strip():
                continue
            symbol = symbol.strip()
            price = _to_float(entry.get("price_usd"))
            column = prices.setdefault(symbol, [None] * len(records))
            # Guard against duplicate symbols within one snapshot: keep first.
            if symbol not in seen:
                column[index] = price
                seen.add(symbol)

    return timestamps, prices


def _return_series(
    timestamps: list[str], prices: dict[str, list[float | None]]
) -> list[AssetReturnSeries]:
    """Compute simple-return series per asset from an aligned price table.

    A return is defined only between two consecutive snapshots where both prices
    are present and the earlier price is strictly positive. The return timestamp
    is the later snapshot's timestamp.
    """
    series: list[AssetReturnSeries] = []
    for symbol in sorted(prices):
        column = prices[symbol]
        ret_ts: list[str] = []
        rets: list[float] = []
        for i in range(1, len(column)):
            prev_price = column[i - 1]
            curr_price = column[i]
            if prev_price is None or curr_price is None:
                continue
            if prev_price <= 0:
                continue
            ret_ts.append(timestamps[i])
            rets.append(curr_price / prev_price - 1.0)
        mean = sum(rets) / len(rets) if rets else 0.0
        series.append(
            AssetReturnSeries(
                symbol=symbol,
                timestamps=tuple(ret_ts),
                returns=tuple(rets),
                mean=mean,
            )
        )
    return series


def _pearson(
    a_ts: tuple[str, ...],
    a_ret: tuple[float, ...],
    b_ts: tuple[str, ...],
    b_ret: tuple[float, ...],
) -> tuple[float, int, bool]:
    """Pearson correlation over the timestamps shared by two return series.

    Returns ``(correlation, n_overlap, defined)``. ``defined`` is ``False``
    (and the coefficient ``0.0``) when fewer than :data:`MIN_PAIRED_RETURNS`
    timestamps overlap or either leg has zero variance over the overlap.
    """
    a_map = dict(zip(a_ts, a_ret))
    b_map = dict(zip(b_ts, b_ret))
    shared = [ts for ts in a_ts if ts in b_map]
    n = len(shared)
    if n < MIN_PAIRED_RETURNS:
        return 0.0, n, False

    xs = [a_map[ts] for ts in shared]
    ys = [b_map[ts] for ts in shared]
    mean_x = sum(xs) / n
    mean_y = sum(ys) / n

    cov = 0.0
    var_x = 0.0
    var_y = 0.0
    for x, y in zip(xs, ys):
        dx = x - mean_x
        dy = y - mean_y
        cov += dx * dy
        var_x += dx * dx
        var_y += dy * dy

    if var_x <= 0.0 or var_y <= 0.0:
        return 0.0, n, False

    corr = cov / math.sqrt(var_x * var_y)
    # Clamp tiny floating-point overshoots into the valid range.
    corr = max(-1.0, min(1.0, corr))
    return corr, n, True


def correlation_matrix(series: list[AssetReturnSeries]) -> CorrelationMatrix:
    """Build the pairwise Pearson correlation matrix from return series.

    Args:
        series: Per-asset return series (e.g. from :func:`_return_series`).

    Returns:
        A :class:`CorrelationMatrix`. Assets with no returns are dropped. When
        fewer than two assets have returns the matrix is empty.
    """
    usable = [s for s in series if s.returns]
    symbols = sorted(s.symbol for s in usable)
    by_symbol = {s.symbol: s for s in usable}

    matrix: dict[str, dict[str, float]] = {
        a: {b: 0.0 for b in symbols} for a in symbols
    }
    for sym in symbols:
        matrix[sym][sym] = 1.0

    pairs: list[CorrelationPair] = []
    for i, sym_a in enumerate(symbols):
        a_series = by_symbol[sym_a]
        for sym_b in symbols[i + 1 :]:
            b_series = by_symbol[sym_b]
            corr, n_overlap, defined = _pearson(
                a_series.timestamps,
                a_series.returns,
                b_series.timestamps,
                b_series.returns,
            )
            matrix[sym_a][sym_b] = corr
            matrix[sym_b][sym_a] = corr
            pairs.append(
                CorrelationPair(
                    a=sym_a,
                    b=sym_b,
                    correlation=corr,
                    observations=n_overlap,
                    defined=defined,
                )
            )

    pairs.sort(
        key=lambda p: (-abs(p.correlation), p.a, p.b)
    )

    # n_observations reflects the longest return series (the most snapshots any
    # one asset spanned), i.e. the depth of the price history.
    n_obs = max((len(s.returns) for s in usable), default=0)
    return CorrelationMatrix(
        symbols=symbols, matrix=matrix, pairs=pairs, n_observations=n_obs
    )


def _normalize_weights(raw: list[tuple[str, float]]) -> ExposureSummary:
    """Build an :class:`ExposureSummary` from ``(symbol, gross_weight)`` pairs.

    Negative weights are treated by gross (absolute) value for concentration.
    Zero/empty input yields an empty summary.
    """
    cleaned = [
        (symbol, abs(weight))
        for symbol, weight in raw
        if symbol and weight is not None
    ]
    cleaned = [(s, w) for s, w in cleaned if w > 0.0]
    if not cleaned:
        return _empty_exposure()

    gross = sum(w for _, w in cleaned)
    if gross <= 0.0:
        return _empty_exposure()

    positions = [
        AssetExposure(symbol=symbol, weight=weight, weight_pct=weight * 100.0)
        for symbol, weight in cleaned
    ]
    positions.sort(key=lambda pos: (-pos.weight, pos.symbol))

    # HHI on the *normalized* fractions so it lands in (0, 1] regardless of
    # whether the raw weights summed to 1.0 or to percentages.
    herfindahl = sum((w / gross) ** 2 for _, w in cleaned)
    effective = 1.0 / herfindahl if herfindahl > 0.0 else 0.0

    return ExposureSummary(
        positions=positions,
        gross_weight=gross,
        herfindahl=herfindahl,
        effective_positions=effective,
        position_count=len(positions),
    )


def exposure_from_report(report: dict | None) -> ExposureSummary:
    """Compute the exposure summary from the agent's run report.

    The latest target book is the run report's ``positions`` list, each item
    carrying ``weight_pct`` (preferred) or ``value_usd`` (fallback, normalized
    against the book). Missing / malformed entries are skipped.

    Args:
        report: Parsed run report dict (e.g. from
            :func:`guardrail_lab.loaders.load_run_report`), or ``None``.

    Returns:
        An :class:`ExposureSummary` (empty when no usable positions exist).
    """
    if not isinstance(report, dict):
        return _empty_exposure()
    positions = report.get("positions")
    if not isinstance(positions, list):
        return _empty_exposure()

    by_pct: list[tuple[str, float]] = []
    by_value: list[tuple[str, float]] = []
    for entry in positions:
        if not isinstance(entry, dict):
            continue
        symbol = entry.get("symbol")
        if not isinstance(symbol, str) or not symbol.strip():
            continue
        symbol = symbol.strip()
        pct = _to_float(entry.get("weight_pct"))
        if pct is not None:
            by_pct.append((symbol, pct / 100.0))
        value = _to_float(entry.get("value_usd"))
        if value is not None:
            by_value.append((symbol, value))

    # Prefer explicit weights; fall back to USD values (which _normalize_weights
    # turns into fractions via the gross total).
    if by_pct:
        return _normalize_weights(by_pct)
    return _normalize_weights(by_value)


def analyze_correlation(
    snapshot_dir: str = DEFAULT_SNAPSHOT_DIR,
    report_path: str = DEFAULT_REPORT,
    db_path: str = DEFAULT_DB,
) -> CorrelationReport:
    """Run the full correlation + exposure analysis over recorded data.

    Reads the per-asset price history from the snapshot directory (preferred) or
    the event log (fallback), derives returns and the Pearson correlation
    matrix, and computes the concentration/exposure summary from the latest
    target book in the run report.

    Args:
        snapshot_dir: Directory of ``*.jsonl`` market snapshots.
        report_path: Path to the agent's JSON run report (latest target book).
        db_path: Path to the SQLite event log (used only for the snapshot
            fallback, which currently carries no per-asset prices).

    Returns:
        A :class:`CorrelationReport`. Never raises: missing or unusable inputs
        are reported via ``ok`` / ``reason`` with empty sub-results.
    """
    # ---- Price/return history (snapshots preferred, event log fallback). ----
    records = _iter_snapshot_records(snapshot_dir)
    source = "snapshots"
    if not records:
        source = "event-log"
        records = _snapshot_records_from_events(db_path)
    if not records:
        source = "none"

    timestamps, prices = _prices_by_asset(records)
    series = _return_series(timestamps, prices)
    matrix = correlation_matrix(series)

    # ---- Exposure from the latest target book. ----
    report = _load_report(report_path)
    exposure = exposure_from_report(report)

    has_corr = len(matrix.symbols) >= 2 and any(p.defined for p in matrix.pairs)
    has_exposure = exposure.position_count > 0

    if has_corr or has_exposure:
        notes: list[str] = []
        if has_corr:
            notes.append(
                f"{len(matrix.symbols)} assets over "
                f"{matrix.n_observations} return step(s) from {source}"
            )
        else:
            notes.append(
                "no usable correlation history "
                f"(source: {source}, "
                f"{len(records)} snapshot(s))"
            )
        if has_exposure:
            notes.append(f"{exposure.position_count} position(s) in target book")
        else:
            notes.append("no target book positions found")
        return CorrelationReport(
            ok=True,
            reason="; ".join(notes),
            source=source,
            correlation=matrix,
            exposure=exposure,
            series=series,
        )

    if not records:
        reason = (
            "no price history — expected snapshots under "
            f"{snapshot_dir}/ or a populated event log at {db_path}, and no "
            f"target book at {report_path}."
        )
    else:
        reason = (
            f"insufficient data — {len(records)} snapshot(s) from {source} did "
            "not yield two correlated assets, and no target book positions were "
            f"found at {report_path}."
        )
    return CorrelationReport(
        ok=False,
        reason=reason,
        source=source,
        correlation=_empty_matrix(len(records)),
        exposure=_empty_exposure(),
        series=series,
    )


def _load_report(report_path: str) -> dict | None:
    """Load the run report via the shared loader, tolerating a missing module."""
    try:
        from guardrail_lab.loaders import load_run_report
    except ImportError:  # pragma: no cover - defensive, package always present
        return None
    return load_run_report(report_path)


def _snapshot_records_from_events(db_path: str) -> list[dict]:
    """Fallback price history from the event log.

    The current ``market_snapshot_received`` schema records only summary counts
    (``{"assets": N, "ts": ...}``) and carries no per-asset prices, so no return
    series can be reconstructed from it. This returns an empty list rather than
    fabricating prices; the surrounding report then degrades to an exposure-only
    (or empty) result with a clear reason. The hook is kept so a future
    price-carrying snapshot event can be wired in here without changing callers.
    """
    try:
        from guardrail_lab.db import load_events
    except ImportError:  # pragma: no cover - defensive
        return []

    events = load_events(db_path)
    records: list[dict] = []
    for event in events:
        if event.get("event_type") != SNAPSHOT_EVENT:
            continue
        payload = event.get("payload")
        if not isinstance(payload, dict):
            continue
        assets = payload.get("assets")
        # Only usable if the payload actually carries a per-asset list with
        # prices; the current schema stores an integer count, which we skip.
        if isinstance(assets, list) and assets:
            records.append(
                {
                    "timestamp_ms": payload.get("ts"),
                    "assets": assets,
                }
            )
    return records
