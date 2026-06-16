"""Strategy optimizer CLI.

Offline (default): grid/random search over the synthetic objective surface — no
API needed. `--api`: rank the named presets via the live backtest endpoint.

Usage:
    python -m optimizer.cli --offline --metric calmar
    python -m optimizer.cli --offline --random 40 --metric sharpe
    python -m optimizer.cli --api --skill momentum-volatility-blend --metric calmar
    python -m optimizer.cli --offline --csv results.csv
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import sys

from . import grid, runner

# Default search space for offline optimization (the params the agent exposes).
DEFAULT_SPACE = {
    "min_score_to_enter": [0.45, 0.55, 0.6, 0.65, 0.75],
    "min_score_to_hold": [0.35, 0.45, 0.5, 0.55],
    "max_positions": [3.0, 5.0, 7.0],
    "rebalance_threshold_pct": [2.0, 3.0, 5.0],
    "target_stable_reserve_pct": [10.0, 15.0, 20.0],
}

PRESETS = ["conservative", "balanced", "aggressive"]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="optimizer", description="Strategy parameter optimizer")
    parser.add_argument("--offline", action="store_true", help="search the synthetic objective (no API)")
    parser.add_argument("--api", action="store_true", help="rank named presets via the live backtest API")
    parser.add_argument("--skill", default="momentum-volatility-blend")
    parser.add_argument("--metric", default="calmar", choices=["calmar", "sharpe"])
    parser.add_argument("--random", type=int, default=0, help="random search with N samples (else full grid)")
    parser.add_argument("--top", type=int, default=10, help="how many results to show")
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--csv", help="write all results to this CSV path")
    parser.add_argument("--api-url", default=os.environ.get("GUARDRAIL_API", "http://127.0.0.1:8080"))
    args = parser.parse_args(argv)

    if args.api:
        return run_api(args)
    # Default to offline.
    return run_offline(args)


def run_offline(args) -> int:
    score = runner.offline_scorer(args.metric)
    if args.random > 0:
        results = grid.random_search(DEFAULT_SPACE, score, args.random, args.seed)
    else:
        results = grid.grid_search(DEFAULT_SPACE, score)

    print(f"offline {args.metric} optimization over {len(results)} parameter sets")
    _print_top(results, args.top, args.metric)
    if args.csv:
        _write_csv(results, args.csv)
        print(f"wrote {len(results)} rows to {args.csv}")
    return 0


def run_api(args) -> int:
    results: list[dict] = []
    for preset in PRESETS:
        try:
            value = runner.api_backtest(args.skill, preset, args.metric, args.api_url)
            results.append({"params": {"preset": preset}, "score": value})
        except RuntimeError as err:
            print(f"  {preset}: {err}", file=sys.stderr)
    if not results:
        print("no preset scored (is the API running?)", file=sys.stderr)
        return 1
    results.sort(key=lambda r: r["score"], reverse=True)
    print(f"api {args.metric} ranking for skill '{args.skill}'")
    _print_top(results, args.top, args.metric)
    return 0


def _print_top(results: list[dict], top: int, metric: str) -> None:
    best = results[0]
    print(f"best {metric} = {best['score']:.4f}  params = {json.dumps(best['params'])}")
    for i, r in enumerate(results[:top], 1):
        print(f"  {i:>2}. {r['score']:>10.4f}  {json.dumps(r['params'])}")


def _write_csv(results: list[dict], path: str) -> None:
    keys = sorted({k for r in results for k in r["params"]})
    with open(path, "w", newline="", encoding="utf-8") as fh:
        writer = csv.writer(fh)
        writer.writerow([*keys, "score"])
        for r in results:
            writer.writerow([*(r["params"].get(k, "") for k in keys), r["score"]])


if __name__ == "__main__":
    raise SystemExit(main())
