"""Chart helpers with optional plotting support.

Plotting libraries (matplotlib) are optional. When they are not installed the
lab degrades to CSV/text output instead of failing at import time.
"""

import csv
from pathlib import Path

from . import metrics
from .attribution import trade_attribution

try:  # optional dependency — degrade gracefully when missing
    import matplotlib  # type: ignore

    matplotlib.use("Agg")
    import matplotlib.pyplot as _plt  # type: ignore

    PLOTTING_AVAILABLE = True
except Exception:  # noqa: BLE001 - any import/runtime failure means no plotting
    _plt = None
    PLOTTING_AVAILABLE = False


def equity_curve_title() -> str:
    return "Guardrail Alpha Equity Curve"


def write_equity_curve_csv(
    series: list[tuple[str, float]], out_path: str
) -> Path:
    """Write a (timestamp, nav_usd) series to CSV. Always available."""
    path = Path(out_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["timestamp", "nav_usd"])
        for timestamp, nav in series:
            writer.writerow([timestamp, f"{nav:.6f}"])
    return path


def write_allocation_csv(
    rows: list[tuple[str, float]], out_path: str
) -> Path:
    """Write a (symbol, weight_pct) allocation series to CSV. Always available."""
    path = Path(out_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["symbol", "weight_pct"])
        for symbol, weight in rows:
            writer.writerow([symbol, f"{weight:.6f}"])
    return path


def write_attribution_csv(
    rows: list[tuple[str, float]], out_path: str
) -> Path:
    """Write a (symbol, total_amount_usd) series to CSV. Always available."""
    path = Path(out_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["symbol", "total_amount_usd"])
        for symbol, amount in rows:
            writer.writerow([symbol, f"{amount:.2f}"])
    return path


def write_drawdown_csv(
    events: list[dict], out_path: str = "data/exports/drawdown.csv"
) -> str:
    """Write the (timestamp, drawdown_pct) series to CSV. Always available.

    Builds the series via :func:`metrics.drawdown_series` from the event log and
    returns the output path as a string.
    """
    series = metrics.drawdown_series(events)
    path = Path(out_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["timestamp", "drawdown_pct"])
        for timestamp, drawdown_pct in series:
            writer.writerow([timestamp, f"{drawdown_pct:.6f}"])
    return str(path)


def plot_drawdown(
    events: list[dict], out_path: str = "data/exports/drawdown.png"
) -> str | None:
    """Render the drawdown series (percent below running peak) to a PNG.

    Builds the series via :func:`metrics.drawdown_series` from the event log.
    Returns the output path as a string on success, or ``None`` when plotting is
    unavailable (matplotlib missing) or there is nothing to plot, so callers can
    fall back to CSV/text output.
    """
    series = metrics.drawdown_series(events)
    if not PLOTTING_AVAILABLE or _plt is None or not series:
        return None

    path = Path(out_path)
    path.parent.mkdir(parents=True, exist_ok=True)

    timestamps = [point[0] for point in series]
    drawdowns = [point[1] for point in series]

    figure, axis = _plt.subplots(figsize=(10, 5))
    axis.fill_between(range(len(drawdowns)), drawdowns, 0.0, color="#c44e52", alpha=0.4)
    axis.plot(range(len(drawdowns)), drawdowns, color="#c44e52", marker="o")
    axis.set_title("Guardrail Alpha Drawdown")
    axis.set_xlabel("Reconciliation #")
    axis.set_ylabel("Drawdown (%)")
    axis.set_xticks(range(len(timestamps)))
    axis.set_xticklabels(timestamps, rotation=45, ha="right", fontsize=6)
    figure.tight_layout()
    figure.savefig(path)
    _plt.close(figure)
    return str(path)


def _to_float(value: object) -> float | None:
    """Best-effort conversion of a payload value (often a decimal string)."""
    if value is None:
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def _allocation_rows(report: dict | None) -> list[tuple[str, float]]:
    """Extract (symbol, weight_pct) pairs from a run report, sorted descending."""
    if not report:
        return []
    positions = report.get("positions") or []
    rows: list[tuple[str, float]] = []
    for position in positions:
        if not isinstance(position, dict):
            continue
        symbol = position.get("symbol")
        if not isinstance(symbol, str) or not symbol.strip():
            continue
        weight = _to_float(position.get("weight_pct"))
        if weight is None:
            continue
        rows.append((symbol.strip(), weight))
    rows.sort(key=lambda item: (-item[1], item[0]))
    return rows


def plot_equity_curve(
    events: list[dict], out_path: str = "data/exports/equity_curve.png"
) -> str | None:
    """Render the equity curve (NAV over time) to a PNG.

    Builds the NAV series via :func:`metrics.nav_series` from the event log.
    Returns the output path as a string on success, or ``None`` when plotting
    is unavailable (matplotlib missing) or there is nothing to plot, so callers
    can fall back to CSV/text output.
    """
    series = metrics.nav_series(events)
    if not PLOTTING_AVAILABLE or _plt is None or not series:
        return None

    path = Path(out_path)
    path.parent.mkdir(parents=True, exist_ok=True)

    timestamps = [point[0] for point in series]
    navs = [point[1] for point in series]

    figure, axis = _plt.subplots(figsize=(10, 5))
    axis.plot(range(len(navs)), navs, marker="o")
    axis.set_title(equity_curve_title())
    axis.set_xlabel("Reconciliation #")
    axis.set_ylabel("NAV (USD)")
    axis.set_xticks(range(len(timestamps)))
    axis.set_xticklabels(timestamps, rotation=45, ha="right", fontsize=6)
    figure.tight_layout()
    figure.savefig(path)
    _plt.close(figure)
    return str(path)


def plot_allocation(
    report: dict | None, out_path: str = "data/exports/allocation.png"
) -> str | None:
    """Render a bar chart of position weight_pct by symbol from a run report.

    Returns the output path as a string on success, or ``None`` when plotting
    is unavailable (matplotlib missing) or there are no positions, so callers
    can fall back to CSV/text output.
    """
    rows = _allocation_rows(report)
    if not PLOTTING_AVAILABLE or _plt is None or not rows:
        return None

    path = Path(out_path)
    path.parent.mkdir(parents=True, exist_ok=True)

    symbols = [symbol for symbol, _ in rows]
    weights = [weight for _, weight in rows]

    figure, axis = _plt.subplots(figsize=(10, 5))
    axis.bar(range(len(weights)), weights, color="#4c72b0")
    axis.set_title("Guardrail Alpha Allocation")
    axis.set_xlabel("Symbol")
    axis.set_ylabel("Weight (%)")
    axis.set_xticks(range(len(symbols)))
    axis.set_xticklabels(symbols, rotation=45, ha="right", fontsize=8)
    figure.tight_layout()
    figure.savefig(path)
    _plt.close(figure)
    return str(path)


def plot_attribution(
    events: list[dict], out_path: str = "data/exports/attribution.png"
) -> str | None:
    """Render a bar chart of trade-attribution totals by destination symbol.

    Builds totals via :func:`attribution.trade_attribution` from the event log.
    Returns the output path as a string on success, or ``None`` when plotting
    is unavailable (matplotlib missing) or there are no confirmed swaps, so
    callers can fall back to CSV/text output.
    """
    attribution = trade_attribution(events)
    rows: list[tuple[str, float]] = []
    for entry in attribution:
        symbol = entry.get("symbol", "UNKNOWN")
        amount = _to_float(entry.get("total_amount_usd")) or 0.0
        rows.append((symbol, amount))

    if not PLOTTING_AVAILABLE or _plt is None or not rows:
        return None

    path = Path(out_path)
    path.parent.mkdir(parents=True, exist_ok=True)

    symbols = [symbol for symbol, _ in rows]
    amounts = [amount for _, amount in rows]

    figure, axis = _plt.subplots(figsize=(10, 5))
    axis.bar(range(len(amounts)), amounts, color="#dd8452")
    axis.set_title("Guardrail Alpha Trade Attribution")
    axis.set_xlabel("Destination Symbol")
    axis.set_ylabel("Total Amount (USD)")
    axis.set_xticks(range(len(symbols)))
    axis.set_xticklabels(symbols, rotation=45, ha="right", fontsize=8)
    figure.tight_layout()
    figure.savefig(path)
    _plt.close(figure)
    return str(path)
