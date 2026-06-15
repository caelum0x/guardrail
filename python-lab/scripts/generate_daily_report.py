#!/usr/bin/env python3
"""Generate the daily Markdown report.

Writes ``python-lab/reports/daily/<date>.md`` when a run report is available,
otherwise falls back to ``data/exports/daily_report.md``.

Run from the repository root:

    python3 python-lab/scripts/generate_daily_report.py

or from python-lab/:

    python3 scripts/generate_daily_report.py

Standard-library only.
"""

import sys
from datetime import datetime, timezone
from pathlib import Path

# Make ``guardrail_lab`` importable regardless of the current directory.
_LAB_ROOT = Path(__file__).resolve().parent.parent
if str(_LAB_ROOT) not in sys.path:
    sys.path.insert(0, str(_LAB_ROOT))

from guardrail_lab.loaders import load_run_report  # noqa: E402
from guardrail_lab.reports import build_daily_report  # noqa: E402

DEFAULT_DB = "data/guardrail_alpha.db"
DEFAULT_REPORT = "data/run_report.json"
FALLBACK_OUT = "data/exports/daily_report.md"


def _output_path(has_run: bool) -> Path:
    if has_run:
        date_str = datetime.now(timezone.utc).strftime("%Y-%m-%d")
        return _LAB_ROOT / "reports" / "daily" / f"{date_str}.md"
    return Path(FALLBACK_OUT)


def main() -> int:
    db_path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_DB
    report_path = sys.argv[2] if len(sys.argv) > 2 else DEFAULT_REPORT

    markdown = build_daily_report(db_path, report_path)
    has_run = load_run_report(report_path) is not None

    out_path = _output_path(has_run)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(markdown, encoding="utf-8")

    print(f"generate_daily_report: wrote {out_path}")
    if not has_run:
        print("  (no run_report.json found — used fallback location)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
