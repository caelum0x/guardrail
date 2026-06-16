"""Monte Carlo CLI.

    python -m montecarlo.cli --gbm --mu 0.0005 --sigma 0.02 --paths 5000 --horizon 180
    python -m montecarlo.cli --bootstrap --db ../../data/guardrail_alpha.db --paths 5000
    python -m montecarlo.cli --demo

Bootstrap reads the agent's NAV series from the event log when --db is given;
otherwise it uses a small built-in sample return series.
"""

from __future__ import annotations

import argparse
import json
import os
import sqlite3
import sys

from . import sim


def _nav_from_db(db_path: str) -> list[float]:
    if not os.path.isfile(db_path):
        return []
    try:
        conn = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
        rows = conn.execute(
            "SELECT payload_json FROM events WHERE event_type='portfolio_reconciled' "
            "ORDER BY timestamp ASC, id ASC"
        ).fetchall()
        conn.close()
    except sqlite3.Error:
        return []
    out: list[float] = []
    for (payload,) in rows:
        try:
            nav = json.loads(payload).get("nav_usd")
            if nav is not None:
                out.append(float(nav))
        except (json.JSONDecodeError, TypeError, ValueError):
            continue
    return out


SAMPLE_RETURNS = [0.012, -0.008, 0.004, 0.021, -0.015, 0.006, -0.003, 0.018, -0.022, 0.009, 0.001, -0.011]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="montecarlo", description="Monte Carlo strategy outcomes")
    parser.add_argument("--gbm", action="store_true", help="geometric Brownian motion engine")
    parser.add_argument("--bootstrap", action="store_true", help="bootstrap engine (default)")
    parser.add_argument("--demo", action="store_true", help="bootstrap over a built-in sample")
    parser.add_argument("--db", help="read the historical NAV series from this event-log DB")
    parser.add_argument("--paths", type=int, default=5000)
    parser.add_argument("--horizon", type=int, default=180)
    parser.add_argument("--start", type=float, default=10_000.0)
    parser.add_argument("--mu", type=float, default=0.0005, help="GBM per-step drift")
    parser.add_argument("--sigma", type=float, default=0.02, help="GBM per-step vol")
    parser.add_argument("--ruin", type=float, default=0.5, help="ruin threshold as a fraction of start")
    parser.add_argument("--seed", type=int, default=1)
    args = parser.parse_args(argv)

    if args.gbm:
        result = sim.gbm(args.mu, args.sigma, args.paths, args.horizon, args.start, args.seed, args.ruin)
    else:
        # bootstrap (default / --demo / --bootstrap)
        history = SAMPLE_RETURNS
        if args.db and not args.demo:
            nav = _nav_from_db(args.db)
            rets = sim.returns_from_equity(nav)
            if len(rets) >= 2:
                history = rets
            else:
                print(f"note: only {len(rets)} returns from {args.db}; using sample series", file=sys.stderr)
        try:
            result = sim.bootstrap(history, args.paths, args.horizon, args.start, args.seed, args.ruin)
        except ValueError as err:
            print(f"error: {err}", file=sys.stderr)
            return 2

    print(json.dumps(result.summary(), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
