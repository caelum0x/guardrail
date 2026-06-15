#!/usr/bin/env python3
"""Export confirmed-swap trade attribution to CSV.

Writes ``data/exports/trade_attribution.csv`` (creating parent directories as
needed) from :func:`guardrail_lab.attribution.trade_attribution` and prints a
short summary.

Run from the repository root:

    python3 python-lab/scripts/export_trade_attribution.py

or from python-lab/:

    python3 scripts/export_trade_attribution.py

Optional positional argument overrides the database path::

    python3 scripts/export_trade_attribution.py [db_path]

Standard-library only.
"""

import csv
import sys
from pathlib import Path

# Make ``guardrail_lab`` importable regardless of the current directory.
_LAB_ROOT = Path(__file__).resolve().parent.parent
if str(_LAB_ROOT) not in sys.path:
    sys.path.insert(0, str(_LAB_ROOT))

from guardrail_lab.attribution import trade_attribution  # noqa: E402
from guardrail_lab.db import load_events  # noqa: E402

DEFAULT_DB = "data/guardrail_alpha.db"
DEFAULT_OUT = "data/exports/trade_attribution.csv"


def main() -> int:
    db_path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_DB

    if not Path(db_path).exists():
        print(f"export_trade_attribution: no database at {db_path} (nothing to export)")
        return 0

    events = load_events(db_path)
    attribution = trade_attribution(events)

    out_path = Path(DEFAULT_OUT)
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

    total_swaps = sum(row.get("count", 0) for row in attribution)
    total_usd = sum(row.get("total_amount_usd", 0.0) for row in attribution)

    print(f"export_trade_attribution: wrote {len(attribution)} rows to {out_path}")
    print(f"  destinations:  {len(attribution)}")
    print(f"  confirmed swaps: {total_swaps}")
    print(f"  total amount:  ${total_usd:,.2f}")
    for row in attribution:
        symbol = row.get("symbol", "UNKNOWN")
        count = row.get("count", 0)
        amount = row.get("total_amount_usd", 0.0)
        print(f"    {symbol:>8}  x{count}  ${amount:,.2f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
