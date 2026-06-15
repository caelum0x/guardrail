#!/usr/bin/env python3
"""Export the equity curve (timestamp, nav_usd) to CSV.

Run from the repository root:

    python3 python-lab/scripts/export_equity_curve.py

or from python-lab/:

    python3 scripts/export_equity_curve.py

Standard-library only.
"""

import sys
from pathlib import Path

# Make ``guardrail_lab`` importable regardless of the current directory.
_LAB_ROOT = Path(__file__).resolve().parent.parent
if str(_LAB_ROOT) not in sys.path:
    sys.path.insert(0, str(_LAB_ROOT))

from guardrail_lab.charts import write_equity_curve_csv  # noqa: E402
from guardrail_lab.db import load_events  # noqa: E402
from guardrail_lab.metrics import max_drawdown, nav_series, trade_count  # noqa: E402

DEFAULT_DB = "data/guardrail_alpha.db"
DEFAULT_OUT = "data/exports/equity_curve.csv"


def main() -> int:
    db_path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_DB

    if not Path(db_path).exists():
        print(f"export_equity_curve: no database at {db_path} (nothing to export)")
        return 0

    events = load_events(db_path)
    series = nav_series(events)

    if not series:
        print("export_equity_curve: no portfolio_reconciled events found")
        return 0

    out_path = write_equity_curve_csv(series, DEFAULT_OUT)

    navs = [nav for _, nav in series]
    drawdown_pct = max_drawdown(navs) * 100.0

    print(f"export_equity_curve: wrote {len(series)} points to {out_path}")
    print(f"  first NAV:    ${navs[0]:,.2f} @ {series[0][0]}")
    print(f"  last NAV:     ${navs[-1]:,.2f} @ {series[-1][0]}")
    print(f"  max drawdown: {drawdown_pct:.2f}%")
    print(f"  trades:       {trade_count(events)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
