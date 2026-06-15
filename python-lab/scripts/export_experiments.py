#!/usr/bin/env python3
"""Export an experiment-comparison table to CSV and print a summary.

Reads the per-experiment JSON files written by the Rust CLI to
``data/experiments/<tag>.json`` and writes a flat comparison CSV to
``data/exports/experiments_comparison.csv``.

Run from the repository root:

    python3 python-lab/scripts/export_experiments.py

or from python-lab/:

    python3 scripts/export_experiments.py

Standard-library only.
"""

import csv
import sys
from pathlib import Path

# Make ``guardrail_lab`` importable regardless of the current directory.
_LAB_ROOT = Path(__file__).resolve().parent.parent
if str(_LAB_ROOT) not in sys.path:
    sys.path.insert(0, str(_LAB_ROOT))

from guardrail_lab.experiments import compare_table, load_experiments  # noqa: E402

DEFAULT_DIR = "data/experiments"
DEFAULT_OUT = "data/exports/experiments_comparison.csv"

CSV_COLUMNS = (
    "tag",
    "return_pct",
    "excess_pct",
    "max_dd_pct",
    "calmar",
    "trades",
    "preset",
    "steps",
    "fear_greed",
)


def _fmt(value: object) -> str:
    """Render a value for the summary table, blanking ``None``."""
    if value is None:
        return "-"
    if isinstance(value, float):
        return f"{value:.3f}"
    return str(value)


def _csv_value(value: object) -> str:
    """Render a value for CSV output, blanking ``None``."""
    if value is None:
        return ""
    return str(value)


def _row_to_csv(row: dict) -> list[str]:
    return [
        _csv_value(row.get("tag")),
        _csv_value(row.get("total_return_pct")),
        _csv_value(row.get("excess_return_pct")),
        _csv_value(row.get("max_drawdown_pct")),
        _csv_value(row.get("calmar_ratio")),
        _csv_value(row.get("trade_count")),
        _csv_value(row.get("preset")),
        _csv_value(row.get("steps")),
        _csv_value(row.get("fear_greed")),
    ]


def _print_summary(rows: list[dict]) -> None:
    headers = ("tag", "return%", "excess%", "max_dd%", "calmar", "trades", "preset")
    table = [
        (
            _fmt(row.get("tag")),
            _fmt(row.get("total_return_pct")),
            _fmt(row.get("excess_return_pct")),
            _fmt(row.get("max_drawdown_pct")),
            _fmt(row.get("calmar_ratio")),
            _fmt(row.get("trade_count")),
            _fmt(row.get("preset")),
        )
        for row in rows
    ]

    widths = [
        max(len(headers[i]), *(len(line[i]) for line in table)) if table
        else len(headers[i])
        for i in range(len(headers))
    ]

    def _line(cells: tuple[str, ...]) -> str:
        return "  ".join(cell.ljust(widths[i]) for i, cell in enumerate(cells))

    print(_line(headers))
    print(_line(tuple("-" * w for w in widths)))
    for line in table:
        print(_line(line))


def main() -> int:
    experiments_dir = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_DIR

    experiments = load_experiments(experiments_dir)
    if not experiments:
        print(
            f"export_experiments: no experiment files found in "
            f"{experiments_dir} (nothing to export)"
        )
        return 0

    rows = compare_table(experiments)

    out_path = Path(DEFAULT_OUT)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(CSV_COLUMNS)
        for row in rows:
            writer.writerow(_row_to_csv(row))

    print(f"export_experiments: wrote {len(rows)} experiments to {out_path}")
    print()
    _print_summary(rows)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
