"""CLI for the risk-analytics service. Pure stdlib — runs without FastAPI/numpy.

Usage:
    python -m risk_analytics.cli metrics <equity.json>   # JSON list of NAV points
    python -m risk_analytics.cli live [--db PATH]         # read the agent event log
    python -m risk_analytics.cli demo                     # a built-in sample curve
"""

from __future__ import annotations

import argparse
import json
import sys

from . import metrics, store


def _print(obj: dict) -> None:
    print(json.dumps(obj, indent=2, default=float))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="risk_analytics", description="Portfolio risk metrics")
    sub = parser.add_subparsers(dest="cmd", required=True)

    m = sub.add_parser("metrics", help="metrics for an equity curve in a JSON file")
    m.add_argument("path", help="path to a JSON array of NAV/equity points")
    m.add_argument("--periods-per-year", type=float, default=metrics.DEFAULT_PERIODS_PER_YEAR)

    live = sub.add_parser("live", help="metrics from the agent's SQLite event log")
    live.add_argument("--db", default=store.DEFAULT_DB)

    sub.add_parser("demo", help="metrics for a built-in sample equity curve")

    args = parser.parse_args(argv)

    if args.cmd == "metrics":
        try:
            with open(args.path, "r", encoding="utf-8") as fh:
                equity = [float(x) for x in json.load(fh)]
        except (OSError, ValueError, json.JSONDecodeError) as err:
            print(f"error: cannot read equity curve: {err}", file=sys.stderr)
            return 2
        if len(equity) < 2:
            print("error: need at least 2 equity points", file=sys.stderr)
            return 2
        _print(metrics.summary(equity, args.periods_per_year))
        return 0

    if args.cmd == "live":
        equity = store.equity_series(args.db)
        if len(equity) < 2:
            print(f"error: only {len(equity)} NAV point(s) in {args.db}; run the agent first", file=sys.stderr)
            return 1
        _print(metrics.summary(equity))
        return 0

    if args.cmd == "demo":
        # A sample curve with a drawdown and recovery.
        equity = [10000, 10200, 10150, 9800, 9600, 9900, 10300, 10250, 10600, 11000]
        _print(metrics.summary(equity))
        return 0

    return 2


if __name__ == "__main__":
    raise SystemExit(main())
