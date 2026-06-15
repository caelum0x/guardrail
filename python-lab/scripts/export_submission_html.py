#!/usr/bin/env python3
"""Export the self-contained HTML submission report for judges.

Builds the report via :func:`guardrail_lab.submission.build_submission_html`
and writes it to ``python-lab/reports/final_submission/submission.html``, then
prints the output path.

Run from the repository root:

    python3 python-lab/scripts/export_submission_html.py

or from python-lab/:

    python3 scripts/export_submission_html.py

Standard-library only.
"""

import sys
from pathlib import Path

# Make ``guardrail_lab`` importable regardless of the current directory.
_LAB_ROOT = Path(__file__).resolve().parent.parent
if str(_LAB_ROOT) not in sys.path:
    sys.path.insert(0, str(_LAB_ROOT))

from guardrail_lab.submission import build_submission_html  # noqa: E402

# Output lives under python-lab/reports so it is independent of the caller's cwd.
DEFAULT_OUT = _LAB_ROOT / "reports" / "final_submission" / "submission.html"


def main() -> int:
    html_doc = build_submission_html()

    out_path = DEFAULT_OUT
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(html_doc, encoding="utf-8")

    print(f"export_submission_html: wrote {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
