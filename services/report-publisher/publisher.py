#!/usr/bin/env python3
"""Guardrail report publisher.

Renders the python-lab HTML report bundle (dossier / journal / ensemble) into a
published directory and writes a small index, so a daily signed report set can be
served statically. Read-only over the event log + run report; it never trades.

It invokes the existing analytics rather than re-implementing them: it imports
``guardrail_lab.report_bundle`` directly when python-lab is importable, otherwise
shells out to ``python3 python-lab/analyze.py bundle``. ``--dry-run`` (default)
prints the plan without writing. Standard library only.

Usage:
    python3 services/report-publisher/publisher.py --dry-run
    python3 services/report-publisher/publisher.py --out reports/published
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


def _repo_root() -> Path:
    """Walk up to the folder containing python-lab/guardrail_lab."""
    here = Path(__file__).resolve()
    for parent in [here.parent, *here.parents]:
        if (parent / "python-lab" / "guardrail_lab").is_dir():
            return parent
    return here.parent.parent.parent


def publish(out_dir: str, db: str, report: str, dry_run: bool) -> int:
    root = _repo_root()
    out = (root / out_dir).resolve()
    db_path = str(root / db)
    report_path = str(root / report)

    if dry_run:
        print("[dry-run] would render the HTML report bundle:")
        print(f"  out:    {out}")
        print(f"  db:     {db_path}")
        print(f"  report: {report_path}")
        print(f"  via:    python3 python-lab/analyze.py bundle --out {out}")
        return 0

    out.mkdir(parents=True, exist_ok=True)
    lab = root / "python-lab"
    # Prefer the in-process builder; fall back to the CLI.
    try:
        sys.path.insert(0, str(lab))
        from guardrail_lab import report_bundle  # type: ignore

        paths = report_bundle.build_bundle(str(out), db_path, report_path)
        print(f"published {len(paths)} report file(s) to {out}")
        for p in paths:
            print(f"  {p}")
        return 0
    except Exception as exc:  # noqa: BLE001 — fall back to the CLI on any import/runtime issue
        print(f"(in-process builder unavailable: {exc}; using analyze.py CLI)", file=sys.stderr)
        proc = subprocess.run(
            [sys.executable, "python-lab/analyze.py", "bundle", "--out", str(out)],
            cwd=str(root),
            check=False,
        )
        return proc.returncode


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Publish the Guardrail HTML report bundle.")
    parser.add_argument("--out", default="reports/published")
    parser.add_argument("--db", default="data/guardrail_alpha.db")
    parser.add_argument("--report", default="data/run_report.json")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--dry-run", dest="dry_run", action="store_true", default=True)
    mode.add_argument("--write", dest="dry_run", action="store_false")
    args = parser.parse_args(argv)
    return publish(args.out, args.db, args.report, args.dry_run)


if __name__ == "__main__":
    raise SystemExit(main())
