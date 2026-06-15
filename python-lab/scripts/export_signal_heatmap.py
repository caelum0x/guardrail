#!/usr/bin/env python3
"""Export a regime timeline + per-symbol final-weight summary to CSV.

Builds a simple "signal summary" from the event log (regime classifications)
and the agent's run report (final position weights), and writes it to
``data/exports/signal_summary.csv``. No matplotlib required.

The CSV has two labelled sections::

    section,key,value
    regime,<timestamp>,<regime>
    ...
    weight,<symbol>,<weight_pct>
    ...

Run from the repository root:

    python3 python-lab/scripts/export_signal_heatmap.py

or from python-lab/:

    python3 scripts/export_signal_heatmap.py

Optional positional argument overrides the database path::

    python3 scripts/export_signal_heatmap.py [db_path]

Standard-library only.
"""

import csv
import sys
from pathlib import Path

# Make ``guardrail_lab`` importable regardless of the current directory.
_LAB_ROOT = Path(__file__).resolve().parent.parent
if str(_LAB_ROOT) not in sys.path:
    sys.path.insert(0, str(_LAB_ROOT))

from guardrail_lab.attribution import regime_timeline  # noqa: E402
from guardrail_lab.db import load_events  # noqa: E402
from guardrail_lab.loaders import load_run_report  # noqa: E402

DEFAULT_DB = "data/guardrail_alpha.db"
DEFAULT_REPORT = "data/run_report.json"
DEFAULT_OUT = "data/exports/signal_summary.csv"


def _to_float(value: object) -> float | None:
    """Best-effort conversion of a payload value (often a decimal string)."""
    if value is None:
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


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


def main() -> int:
    db_path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_DB

    if not Path(db_path).exists():
        print(f"export_signal_heatmap: no database at {db_path} (nothing to export)")
        return 0

    events = load_events(db_path)
    report = load_run_report(DEFAULT_REPORT)

    timeline = regime_timeline(events)
    weights = _final_weights(report)

    out_path = Path(DEFAULT_OUT)
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

    regime_counts: dict[str, int] = {}
    for entry in timeline:
        regime = entry.get("regime", "unknown")
        regime_counts[regime] = regime_counts.get(regime, 0) + 1

    print(f"export_signal_heatmap: wrote {out_path}")
    print(f"  regime classifications: {len(timeline)}")
    if timeline:
        print(
            f"    first: {timeline[0].get('regime', 'unknown')} "
            f"@ {timeline[0].get('timestamp', '')}"
        )
        print(
            f"    last:  {timeline[-1].get('regime', 'unknown')} "
            f"@ {timeline[-1].get('timestamp', '')}"
        )
        for regime, count in sorted(regime_counts.items()):
            print(f"    {regime}: {count}")
    print(f"  final positions: {len(weights)}")
    for symbol, weight in weights:
        print(f"    {symbol:>8}  {weight:.2f}%")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
