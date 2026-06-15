#!/usr/bin/env python3
"""Export the drawdown chart (percent below running NAV peak).

Renders a PNG when matplotlib is installed. When matplotlib is missing the
underlying (timestamp, drawdown_pct) series is written to CSV instead and a
notice is printed, so the script always succeeds.

Run from the repository root:

    python3 python-lab/scripts/export_drawdown_chart.py

or from python-lab/:

    python3 scripts/export_drawdown_chart.py

Optional positional argument overrides the database path::

    python3 scripts/export_drawdown_chart.py [db_path]

Standard-library only (matplotlib is optional).
"""

import sys
from pathlib import Path

# Make ``guardrail_lab`` importable regardless of the current directory.
_LAB_ROOT = Path(__file__).resolve().parent.parent
if str(_LAB_ROOT) not in sys.path:
    sys.path.insert(0, str(_LAB_ROOT))

from guardrail_lab import charts  # noqa: E402
from guardrail_lab.db import load_events  # noqa: E402
from guardrail_lab.metrics import drawdown_series  # noqa: E402

DEFAULT_DB = "data/guardrail_alpha.db"
DRAWDOWN_PNG = "data/exports/drawdown.png"
DRAWDOWN_CSV = "data/exports/drawdown.csv"


def main() -> int:
    db_path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_DB

    if not Path(db_path).exists():
        print(f"export_drawdown_chart: no database at {db_path} (nothing to export)")
        return 0

    events = load_events(db_path)
    series = drawdown_series(events)

    if not series:
        print("export_drawdown_chart: no portfolio_reconciled events found")
        return 0

    png = charts.plot_drawdown(events, DRAWDOWN_PNG)
    if png is not None:
        artifact = png
        print(f"export_drawdown_chart: wrote PNG  {artifact}")
    else:
        artifact = charts.write_drawdown_csv(events, DRAWDOWN_CSV)
        print(
            f"export_drawdown_chart: matplotlib unavailable, wrote CSV  {artifact}"
        )

    drawdowns = [drawdown_pct for _, drawdown_pct in series]
    max_dd = min(drawdowns)  # most negative is the worst drawdown
    worst_index = drawdowns.index(max_dd)

    print(f"  points:       {len(series)}")
    print(f"  max drawdown: {max_dd:.2f}% @ {series[worst_index][0]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
