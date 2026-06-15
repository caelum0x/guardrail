#!/usr/bin/env python3
"""Generate the judge-facing Markdown submission report.

Writes ``python-lab/reports/final_submission/submission.md`` (creating parent
directories as needed) using :func:`guardrail_lab.reports.build_submission_report`.

Run from the repository root:

    python3 python-lab/scripts/generate_submission_report.py

or from python-lab/:

    python3 scripts/generate_submission_report.py

Optional positional arguments override the defaults::

    python3 scripts/generate_submission_report.py [db_path] [report_path]

Standard-library only.
"""

import sys
from pathlib import Path

# Make ``guardrail_lab`` importable regardless of the current directory.
_LAB_ROOT = Path(__file__).resolve().parent.parent
if str(_LAB_ROOT) not in sys.path:
    sys.path.insert(0, str(_LAB_ROOT))

from guardrail_lab.reports import build_submission_report  # noqa: E402

DEFAULT_DB = "data/guardrail_alpha.db"
DEFAULT_REPORT = "data/run_report.json"
OUT_PATH = _LAB_ROOT / "reports" / "final_submission" / "submission.md"


def main() -> int:
    db_path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_DB
    report_path = sys.argv[2] if len(sys.argv) > 2 else DEFAULT_REPORT

    markdown = build_submission_report(db_path, report_path)

    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUT_PATH.write_text(markdown, encoding="utf-8")

    print(f"generate_submission_report: wrote {OUT_PATH}")
    if not Path(db_path).exists():
        print(f"  (no database at {db_path} — report uses placeholders)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
