#!/usr/bin/env python3
"""Run the full Guardrail Alpha analytics export pipeline.

Invokes the same underlying library calls the individual export scripts use --
equity curve, drawdown, trade attribution, signal summary, and the daily
Markdown report -- by importing and calling them directly (no shelling out).

PNG charts are rendered when matplotlib is installed; otherwise the data is
written to CSV instead, so the pipeline always succeeds.

At the end a manifest of every artifact written under ``data/exports`` and
``python-lab/reports`` is printed.

Run from the repository root:

    python3 python-lab/scripts/export_all.py

or from python-lab/:

    python3 scripts/export_all.py

Optional positional argument overrides the database path::

    python3 scripts/export_all.py [db_path]

Standard-library only (matplotlib is optional).
"""

import csv
import sys
from datetime import datetime, timezone
from pathlib import Path

# Make ``guardrail_lab`` importable regardless of the current directory.
_LAB_ROOT = Path(__file__).resolve().parent.parent
if str(_LAB_ROOT) not in sys.path:
    sys.path.insert(0, str(_LAB_ROOT))

from guardrail_lab import charts  # noqa: E402
from guardrail_lab.attribution import regime_timeline, trade_attribution  # noqa: E402
from guardrail_lab.db import load_events  # noqa: E402
from guardrail_lab.loaders import load_run_report  # noqa: E402
from guardrail_lab.metrics import drawdown_series, nav_series  # noqa: E402
from guardrail_lab.reports import build_daily_report  # noqa: E402

DEFAULT_DB = "data/guardrail_alpha.db"
DEFAULT_REPORT = "data/run_report.json"

EQUITY_PNG = "data/exports/equity_curve.png"
EQUITY_CSV = "data/exports/equity_curve.csv"
ALLOCATION_PNG = "data/exports/allocation.png"
ALLOCATION_CSV = "data/exports/allocation.csv"
DRAWDOWN_PNG = "data/exports/drawdown.png"
DRAWDOWN_CSV = "data/exports/drawdown.csv"
ATTRIBUTION_CSV = "data/exports/trade_attribution.csv"
SIGNAL_SUMMARY_CSV = "data/exports/signal_summary.csv"
DAILY_FALLBACK = "data/exports/daily_report.md"


def _to_float(value: object) -> float | None:
    """Best-effort conversion of a payload value (often a decimal string)."""
    if value is None:
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def _export_equity(events: list[dict]) -> str | None:
    """Render the equity-curve PNG, or fall back to CSV."""
    series = nav_series(events)
    if not series:
        print("export_all: equity curve     - no NAV data (skipped)")
        return None

    png = charts.plot_equity_curve(events, EQUITY_PNG)
    if png is not None:
        print(f"export_all: equity curve     - wrote PNG  {png}")
        return png

    csv_path = str(charts.write_equity_curve_csv(series, EQUITY_CSV))
    print(f"export_all: equity curve     - matplotlib off, wrote CSV  {csv_path}")
    return csv_path


def _export_allocation(report: dict | None) -> str | None:
    """Render the allocation PNG, or fall back to CSV."""
    rows = charts._allocation_rows(report)
    if not rows:
        print("export_all: allocation       - no position data (skipped)")
        return None

    png = charts.plot_allocation(report, ALLOCATION_PNG)
    if png is not None:
        print(f"export_all: allocation       - wrote PNG  {png}")
        return png

    csv_path = str(charts.write_allocation_csv(rows, ALLOCATION_CSV))
    print(f"export_all: allocation       - matplotlib off, wrote CSV  {csv_path}")
    return csv_path


def _export_drawdown(events: list[dict]) -> str | None:
    """Render the drawdown PNG, or fall back to CSV."""
    series = drawdown_series(events)
    if not series:
        print("export_all: drawdown         - no NAV data (skipped)")
        return None

    png = charts.plot_drawdown(events, DRAWDOWN_PNG)
    if png is not None:
        print(f"export_all: drawdown         - wrote PNG  {png}")
        return png

    csv_path = charts.write_drawdown_csv(events, DRAWDOWN_CSV)
    print(f"export_all: drawdown         - matplotlib off, wrote CSV  {csv_path}")
    return csv_path


def _export_attribution(events: list[dict]) -> str | None:
    """Write the trade-attribution CSV (always available)."""
    attribution = trade_attribution(events)
    if not attribution:
        print("export_all: trade attribution - no confirmed swaps (skipped)")
        return None

    out_path = Path(ATTRIBUTION_CSV)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["symbol", "count", "total_amount_usd"])
        for row in attribution:
            writer.writerow(
                [
                    row.get("symbol", "UNKNOWN"),
                    row.get("count", 0),
                    f"{row.get('total_amount_usd', 0.0):.2f}",
                ]
            )
    print(f"export_all: trade attribution - wrote CSV  {out_path}")
    return str(out_path)


def _final_weights(report: dict | None) -> list[tuple[str, float]]:
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


def _export_signal_summary(events: list[dict], report: dict | None) -> str | None:
    """Write the regime + final-weight signal summary CSV (always available)."""
    timeline = regime_timeline(events)
    weights = _final_weights(report)
    if not timeline and not weights:
        print("export_all: signal summary    - no regime/weight data (skipped)")
        return None

    out_path = Path(SIGNAL_SUMMARY_CSV)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["section", "key", "value"])
        for entry in timeline:
            writer.writerow(
                ["regime", entry.get("timestamp", ""), entry.get("regime", "unknown")]
            )
        for symbol, weight in weights:
            writer.writerow(["weight", symbol, f"{weight:.6f}"])
    print(f"export_all: signal summary    - wrote CSV  {out_path}")
    return str(out_path)


def _export_daily_report(db_path: str, report_path: str) -> str:
    """Build the daily Markdown report and write it to the correct location."""
    markdown = build_daily_report(db_path, report_path)
    has_run = load_run_report(report_path) is not None

    if has_run:
        date_str = datetime.now(timezone.utc).strftime("%Y-%m-%d")
        out_path = _LAB_ROOT / "reports" / "daily" / f"{date_str}.md"
    else:
        out_path = Path(DAILY_FALLBACK)

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(markdown, encoding="utf-8")
    note = "" if has_run else " (no run_report.json — fallback location)"
    print(f"export_all: daily report      - wrote  {out_path}{note}")
    return str(out_path)


def main() -> int:
    db_path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_DB

    if not Path(db_path).exists():
        print(f"export_all: no database at {db_path} (nothing to export)")
        return 0

    if charts.PLOTTING_AVAILABLE:
        print("export_all: matplotlib available - rendering PNG charts")
    else:
        print("export_all: matplotlib NOT installed - writing CSV fallbacks")

    events = load_events(db_path)
    report = load_run_report(DEFAULT_REPORT)

    artifacts: list[str | None] = [
        _export_equity(events),
        _export_allocation(report),
        _export_drawdown(events),
        _export_attribution(events),
        _export_signal_summary(events, report),
        _export_daily_report(db_path, DEFAULT_REPORT),
    ]

    written = [artifact for artifact in artifacts if artifact]

    print("")
    print(f"export_all: manifest ({len(written)} artifact(s))")
    for artifact in written:
        print(f"  - {artifact}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
