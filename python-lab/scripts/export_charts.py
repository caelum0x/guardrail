#!/usr/bin/env python3
"""Export all Guardrail Alpha charts (equity curve, allocation, attribution).

Renders PNG charts when matplotlib is installed. When matplotlib is missing the
underlying data is written to CSV instead and a notice is printed, so the script
always succeeds.

Run from the repository root:

    python3 python-lab/scripts/export_charts.py

or from python-lab/:

    python3 scripts/export_charts.py

Optional positional argument overrides the database path::

    python3 scripts/export_charts.py [db_path]

Standard-library only (matplotlib is optional).
"""

import sys
from pathlib import Path

# Make ``guardrail_lab`` importable regardless of the current directory.
_LAB_ROOT = Path(__file__).resolve().parent.parent
if str(_LAB_ROOT) not in sys.path:
    sys.path.insert(0, str(_LAB_ROOT))

from guardrail_lab import charts  # noqa: E402
from guardrail_lab.attribution import trade_attribution  # noqa: E402
from guardrail_lab.db import load_events  # noqa: E402
from guardrail_lab.loaders import load_run_report  # noqa: E402
from guardrail_lab.metrics import nav_series  # noqa: E402

DEFAULT_DB = "data/guardrail_alpha.db"
DEFAULT_REPORT = "data/run_report.json"

EQUITY_PNG = "data/exports/equity_curve.png"
EQUITY_CSV = "data/exports/equity_curve.csv"
ALLOCATION_PNG = "data/exports/allocation.png"
ALLOCATION_CSV = "data/exports/allocation.csv"
ATTRIBUTION_PNG = "data/exports/attribution.png"
ATTRIBUTION_CSV = "data/exports/attribution.csv"


def _export_equity(events: list[dict]) -> str | None:
    """Render the equity-curve PNG, or fall back to CSV. Returns artifact path."""
    series = nav_series(events)
    if not series:
        print("export_charts: equity curve   - no NAV data (skipped)")
        return None

    png = charts.plot_equity_curve(events, EQUITY_PNG)
    if png is not None:
        print(f"export_charts: equity curve   - wrote PNG  {png}")
        return png

    csv_path = charts.write_equity_curve_csv(series, EQUITY_CSV)
    print(
        f"export_charts: equity curve   - matplotlib unavailable, "
        f"wrote CSV  {csv_path}"
    )
    return str(csv_path)


def _export_allocation(report: dict | None) -> str | None:
    """Render the allocation PNG, or fall back to CSV. Returns artifact path."""
    rows = charts._allocation_rows(report)
    if not rows:
        print("export_charts: allocation     - no position data (skipped)")
        return None

    png = charts.plot_allocation(report, ALLOCATION_PNG)
    if png is not None:
        print(f"export_charts: allocation     - wrote PNG  {png}")
        return png

    csv_path = charts.write_allocation_csv(rows, ALLOCATION_CSV)
    print(
        f"export_charts: allocation     - matplotlib unavailable, "
        f"wrote CSV  {csv_path}"
    )
    return str(csv_path)


def _export_attribution(events: list[dict]) -> str | None:
    """Render the attribution PNG, or fall back to CSV. Returns artifact path."""
    attribution = trade_attribution(events)
    rows = [
        (
            entry.get("symbol", "UNKNOWN"),
            float(entry.get("total_amount_usd", 0.0) or 0.0),
        )
        for entry in attribution
    ]
    if not rows:
        print("export_charts: attribution    - no confirmed swaps (skipped)")
        return None

    png = charts.plot_attribution(events, ATTRIBUTION_PNG)
    if png is not None:
        print(f"export_charts: attribution    - wrote PNG  {png}")
        return png

    csv_path = charts.write_attribution_csv(rows, ATTRIBUTION_CSV)
    print(
        f"export_charts: attribution    - matplotlib unavailable, "
        f"wrote CSV  {csv_path}"
    )
    return str(csv_path)


def main() -> int:
    db_path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_DB

    if not Path(db_path).exists():
        print(f"export_charts: no database at {db_path} (nothing to export)")
        return 0

    if charts.PLOTTING_AVAILABLE:
        print("export_charts: matplotlib available - rendering PNG charts")
    else:
        print(
            "export_charts: matplotlib NOT installed - "
            "writing CSV fallbacks instead"
        )

    events = load_events(db_path)
    report = load_run_report(DEFAULT_REPORT)

    artifacts = [
        _export_equity(events),
        _export_allocation(report),
        _export_attribution(events),
    ]

    written = [artifact for artifact in artifacts if artifact]
    print(f"export_charts: {len(written)} artifact(s) written")
    for artifact in written:
        print(f"  - {artifact}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
