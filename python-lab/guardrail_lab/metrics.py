"""Metrics derived from the event log.

Standard-library only.
"""


def _to_float(value: object) -> float | None:
    """Best-effort conversion of a payload value (often a decimal string)."""
    if value is None:
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def max_drawdown(values: list[float]) -> float:
    """Largest peak-to-trough decline as a negative fraction (0.0 if none)."""
    peak = float("-inf")
    worst = 0.0
    for value in values:
        peak = max(peak, value)
        if peak > 0:
            worst = min(worst, (value - peak) / peak)
    return worst


def nav_series(events: list[dict]) -> list[tuple[str, float]]:
    """Extract (timestamp, nav_usd) points from ``portfolio_reconciled`` events.

    ``nav_usd`` payloads are high-precision decimal strings; they are parsed to
    float. Points missing a usable NAV are skipped. Ordered by timestamp.
    """
    series: list[tuple[str, float]] = []
    for event in events:
        if event.get("event_type") != "portfolio_reconciled":
            continue
        payload = event.get("payload") or {}
        nav = _to_float(payload.get("nav_usd"))
        timestamp = event.get("timestamp")
        if nav is None or not timestamp:
            continue
        series.append((timestamp, nav))

    series.sort(key=lambda point: point[0])
    return series


def drawdown_series(events: list[dict]) -> list[tuple[str, float]]:
    """Compute a (timestamp, drawdown_pct) series from the NAV history.

    Builds the NAV series via :func:`nav_series`, then tracks the running peak
    and reports the percentage decline from that peak at each point. A fresh
    high reports ``0.0``; a value below the running peak reports a negative
    percentage (e.g. ``-4.32`` for a 4.32% drawdown). Returns an empty list when
    there are no NAV points. Peaks at or below zero are skipped to avoid
    division by zero.
    """
    series = nav_series(events)
    result: list[tuple[str, float]] = []
    peak = float("-inf")
    for timestamp, nav in series:
        peak = max(peak, nav)
        if peak > 0:
            drawdown_pct = (nav - peak) / peak * 100.0
        else:
            drawdown_pct = 0.0
        result.append((timestamp, drawdown_pct))
    return result


def trade_count(events: list[dict]) -> int:
    """Number of confirmed on-chain trades (``tx_confirmed`` events)."""
    return sum(1 for event in events if event.get("event_type") == "tx_confirmed")
