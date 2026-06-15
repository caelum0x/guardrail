#!/usr/bin/env python3
"""Backtest-validate the Track-2 strategy skill examples.

Loads the JSON example files shipped with the
``cmc-regime-routed-alpha`` skill, runs lightweight validation on each, prints
a per-example PASS/ISSUES report plus a summary, and writes a flat CSV to
``data/exports/skill_validation.csv`` (columns: example, issues_count, issues).

Validation issues are reported, not fatal: the script exits nonzero only if a
required input cannot be read (i.e. the examples directory is missing).

Run from the repository root:

    python3 python-lab/scripts/validate_skill.py

or from python-lab/:

    python3 scripts/validate_skill.py

Standard-library only.
"""

import csv
import sys
from pathlib import Path

# Make ``guardrail_lab`` importable regardless of the current directory.
_LAB_ROOT = Path(__file__).resolve().parent.parent
if str(_LAB_ROOT) not in sys.path:
    sys.path.insert(0, str(_LAB_ROOT))

from guardrail_lab.skill import load_skill_examples, validate_example  # noqa: E402

DEFAULT_DIR = "skills/cmc-regime-routed-alpha/examples"
DEFAULT_OUT = "data/exports/skill_validation.csv"

CSV_COLUMNS = ("example", "issues_count", "issues")


def _example_name(example: dict, fallback: str) -> str:
    """Pick a stable display name for an example."""
    source = example.get("_source")
    if isinstance(source, str) and source:
        return source
    return fallback


def _print_report(rows: list[dict]) -> None:
    """Print a per-example PASS/ISSUES report."""
    for row in rows:
        if row["issues"]:
            print(f"ISSUES  {row['example']}")
            for issue in row["issues"]:
                print(f"          - {issue}")
        else:
            print(f"PASS    {row['example']}")


def main() -> int:
    skill_dir = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_DIR

    base = Path(skill_dir)
    if not base.is_dir():
        print(
            f"validate_skill: examples directory not found: {skill_dir}",
            file=sys.stderr,
        )
        return 1

    examples = load_skill_examples(skill_dir)

    rows: list[dict] = []
    for index, example in enumerate(examples):
        issues = validate_example(example)
        rows.append(
            {
                "example": _example_name(example, f"example_{index}"),
                "issues": issues,
            }
        )

    out_path = Path(DEFAULT_OUT)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(CSV_COLUMNS)
        for row in rows:
            writer.writerow(
                [row["example"], len(row["issues"]), "; ".join(row["issues"])]
            )

    _print_report(rows)

    total = len(rows)
    passed = sum(1 for row in rows if not row["issues"])
    failed = total - passed
    print()
    print(f"validate_skill: {passed}/{total} examples passed, {failed} with issues")
    print(f"validate_skill: wrote report to {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
