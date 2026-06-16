"""Command-line interface for the Guardrail report generator.

Examples::

    python -m reporting.cli --db ../../data/guardrail_alpha.db --out report.html
    python -m reporting.cli --db ../../data/guardrail_alpha.db --format text
    python -m reporting.cli --db data.db --report run_report.json --out r.html

Pure standard library: argparse only.
"""

from __future__ import annotations

import argparse
import os
import sys
from typing import Optional, Sequence

from .data import load_event_log, load_run_report
from .html import render_html
from .metrics import compute_metrics
from .text import render_text


def _default_report_path(db_path: str) -> Optional[str]:
    """Guess a sibling run_report.json next to the database, if present."""
    candidate = os.path.join(os.path.dirname(os.path.abspath(db_path)), "run_report.json")
    return candidate if os.path.isfile(candidate) else None


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="reporting.cli",
        description="Generate a self-contained HTML (or text) report from the "
        "Guardrail SQLite event log and run report.",
    )
    parser.add_argument(
        "--db",
        required=True,
        help="Path to the SQLite event log (e.g. data/guardrail_alpha.db).",
    )
    parser.add_argument(
        "--report",
        default=None,
        help="Path to run_report.json. Defaults to a sibling of --db if found. "
        "Use --no-report to skip entirely.",
    )
    parser.add_argument(
        "--no-report",
        action="store_true",
        help="Do not read any run report, even if one exists beside --db.",
    )
    parser.add_argument(
        "--run-id",
        default=None,
        help="Restrict the event log to a single run_id.",
    )
    parser.add_argument(
        "--format",
        choices=("html", "text"),
        default="html",
        help="Output format (default: html).",
    )
    parser.add_argument(
        "--out",
        default=None,
        help="Output file path. Defaults to stdout. For HTML, an --out is "
        "recommended (e.g. report.html).",
    )
    parser.add_argument(
        "--title",
        default="Guardrail Run Report",
        help="Document title for HTML output.",
    )
    return parser


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    # Resolve the run report path.
    if args.no_report:
        report_path = None
    elif args.report is not None:
        report_path = args.report
    else:
        report_path = _default_report_path(args.db)

    try:
        event_log = load_event_log(args.db, run_id=args.run_id)
        run_report = load_run_report(report_path)
    except (FileNotFoundError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    metrics = compute_metrics(event_log, run_report)

    if args.format == "html":
        output = render_html(
            event_log, metrics, run_report, title=args.title
        )
    else:
        output = render_text(event_log, metrics, run_report)

    if args.out:
        out_dir = os.path.dirname(os.path.abspath(args.out))
        os.makedirs(out_dir, exist_ok=True)
        with open(args.out, "w", encoding="utf-8") as fh:
            fh.write(output)
        size = os.path.getsize(args.out)
        print(
            f"wrote {args.format} report to {args.out} "
            f"({size:,} bytes, {metrics.nav_points} NAV points, "
            f"{event_log.total_events} events)",
            file=sys.stderr,
        )
    else:
        sys.stdout.write(output)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
