#!/usr/bin/env python3
"""Stdlib-only smoke check for the Guardrail Alpha analytics layer.

Verifies that the ``guardrail_lab`` package imports cleanly and that the export
pipeline functions return sane shapes. When the event-log database is present
the pipeline is exercised end to end against real data; when it is absent only
the import-and-signature checks run and the script still passes (with a notice).

The script prints a PASS/FAIL checklist and exits non-zero on any failure.

Run from the repository root:

    python3 python-lab/scripts/verify.py

or from python-lab/:

    python3 scripts/verify.py

Optional positional argument overrides the database path::

    python3 scripts/verify.py [db_path]

Standard-library only (matplotlib is optional and never required).
"""

import sys
import tempfile
from pathlib import Path

# Make ``guardrail_lab`` importable regardless of the current directory.
_LAB_ROOT = Path(__file__).resolve().parent.parent
if str(_LAB_ROOT) not in sys.path:
    sys.path.insert(0, str(_LAB_ROOT))

DEFAULT_DB = "data/guardrail_alpha.db"
DEFAULT_REPORT = "data/run_report.json"


class CheckLog:
    """Accumulate PASS/FAIL/INFO lines and track overall success."""

    def __init__(self) -> None:
        self.lines: list[str] = []
        self.ok = True

    def passed(self, message: str) -> None:
        self.lines.append(f"  [PASS] {message}")

    def failed(self, message: str) -> None:
        self.ok = False
        self.lines.append(f"  [FAIL] {message}")

    def info(self, message: str) -> None:
        self.lines.append(f"  [INFO] {message}")

    def check(self, message: str, condition: bool) -> bool:
        """Record PASS when ``condition`` is truthy, otherwise FAIL."""
        if condition:
            self.passed(message)
        else:
            self.failed(message)
        return bool(condition)

    def render(self) -> str:
        return "\n".join(self.lines)


def _resolve_db_path(db_path: str) -> Path:
    """Return the DB path, falling back to a lab-root-relative location.

    Scripts use ``data/...`` relative to the current working directory. When the
    check is launched from elsewhere, also probe the path relative to the lab
    root so the data-backed checks still run when a database exists.
    """
    candidate = Path(db_path)
    if candidate.exists():
        return candidate
    lab_relative = _LAB_ROOT / db_path
    if lab_relative.exists():
        return lab_relative
    return candidate


def _check_imports(log: CheckLog) -> bool:
    """Import every ``guardrail_lab`` module and key callables."""
    try:
        from guardrail_lab import (  # noqa: F401
            attribution,
            charts,
            db,
            loaders,
            metrics,
            reports,
        )
    except Exception as error:  # noqa: BLE001 - report any import failure
        log.failed(f"import guardrail_lab modules ({error!r})")
        return False

    log.passed("import guardrail_lab.{db,loaders,metrics,attribution,charts,reports}")

    callables = {
        "db.load_events": getattr(db, "load_events", None),
        "db.event_counts": getattr(db, "event_counts", None),
        "loaders.load_run_report": getattr(loaders, "load_run_report", None),
        "metrics.nav_series": getattr(metrics, "nav_series", None),
        "metrics.drawdown_series": getattr(metrics, "drawdown_series", None),
        "metrics.max_drawdown": getattr(metrics, "max_drawdown", None),
        "metrics.trade_count": getattr(metrics, "trade_count", None),
        "attribution.trade_attribution": getattr(
            attribution, "trade_attribution", None
        ),
        "attribution.regime_timeline": getattr(
            attribution, "regime_timeline", None
        ),
        "reports.build_daily_report": getattr(reports, "build_daily_report", None),
        "reports.build_submission_report": getattr(
            reports, "build_submission_report", None
        ),
        "charts.write_equity_curve_csv": getattr(
            charts, "write_equity_curve_csv", None
        ),
    }
    all_callable = True
    for name, value in callables.items():
        if not callable(value):
            log.failed(f"expected callable {name}")
            all_callable = False
    if all_callable:
        log.passed(f"all {len(callables)} key functions are callable")

    log.info(
        "matplotlib available - PNG charts"
        if charts.PLOTTING_AVAILABLE
        else "matplotlib not installed - CSV fallback mode"
    )
    return all_callable


def _check_import_only(log: CheckLog) -> None:
    """Exercise the pipeline against an empty event log (no database).

    Every function must tolerate an empty input and return the correct type,
    so the analytics layer is verifiably safe even before the agent has run.
    """
    from guardrail_lab.attribution import regime_timeline, trade_attribution
    from guardrail_lab.db import event_counts
    from guardrail_lab.metrics import (
        drawdown_series,
        max_drawdown,
        nav_series,
        trade_count,
    )

    empty: list[dict] = []
    log.check("nav_series([]) is a list", isinstance(nav_series(empty), list))
    log.check(
        "drawdown_series([]) is a list", isinstance(drawdown_series(empty), list)
    )
    log.check("trade_count([]) == 0", trade_count(empty) == 0)
    log.check(
        "max_drawdown([]) is a float", isinstance(max_drawdown([]), float)
    )
    log.check(
        "trade_attribution([]) is a list",
        isinstance(trade_attribution(empty), list),
    )
    log.check(
        "regime_timeline([]) is a list",
        isinstance(regime_timeline(empty), list),
    )
    log.check("event_counts([]) is a dict", isinstance(event_counts(empty), dict))


def _check_pipeline(log: CheckLog, db_path: Path) -> None:
    """Exercise the export pipeline against a real database."""
    from guardrail_lab.attribution import trade_attribution
    from guardrail_lab.db import load_events
    from guardrail_lab.metrics import drawdown_series, nav_series
    from guardrail_lab.reports import build_daily_report

    db_str = str(db_path)
    report_str = str(db_path.parent / "run_report.json")

    events = load_events(db_str)
    log.check("load_events returns a list", isinstance(events, list))
    log.info(f"loaded {len(events)} event(s) from {db_str}")

    nav = nav_series(events)
    log.check(
        "nav_series returns list of (str, float) tuples",
        isinstance(nav, list)
        and all(
            isinstance(point, tuple)
            and len(point) == 2
            and isinstance(point[0], str)
            and isinstance(point[1], float)
            for point in nav
        ),
    )

    drawdown = drawdown_series(events)
    log.check(
        "drawdown_series returns list of (str, float) tuples",
        isinstance(drawdown, list)
        and all(
            isinstance(point, tuple)
            and len(point) == 2
            and isinstance(point[0], str)
            and isinstance(point[1], float)
            for point in drawdown
        ),
    )

    attribution = trade_attribution(events)
    log.check(
        "trade_attribution returns list of summary dicts",
        isinstance(attribution, list)
        and all(
            isinstance(row, dict)
            and "symbol" in row
            and "count" in row
            and "total_amount_usd" in row
            for row in attribution
        ),
    )

    markdown = build_daily_report(db_str, report_str)
    log.check(
        "build_daily_report returns a non-empty Markdown string",
        isinstance(markdown, str) and markdown.strip().startswith("#"),
    )


def _check_artifact_write(log: CheckLog) -> None:
    """Write an export artifact to a temp dir and confirm it lands on disk."""
    from guardrail_lab.charts import write_equity_curve_csv

    with tempfile.TemporaryDirectory() as tmp:
        out_path = Path(tmp) / "exports" / "equity_curve.csv"
        sample: list[tuple[str, float]] = [
            ("2026-06-13T00:00:00Z", 10000.0),
            ("2026-06-13T01:00:00Z", 10125.5),
        ]
        written = write_equity_curve_csv(sample, str(out_path))
        log.check(
            "write_equity_curve_csv created the file",
            Path(written).exists() and Path(written).stat().st_size > 0,
        )
        contents = Path(written).read_text(encoding="utf-8")
        log.check(
            "artifact CSV has the expected header",
            contents.splitlines()[0] == "timestamp,nav_usd",
        )


def main() -> int:
    db_path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_DB
    resolved_db = _resolve_db_path(db_path)

    log = CheckLog()

    print("Guardrail Alpha analytics smoke check")
    print("=" * 40)
    print("Imports & signatures:")
    imports_ok = _check_imports(log)

    if imports_ok:
        print(log.render())
        log.lines.clear()

        print("Pipeline (empty event log):")
        _check_import_only(log)
        print(log.render())
        log.lines.clear()

        if resolved_db.exists():
            print(f"Pipeline (database: {resolved_db}):")
            _check_pipeline(log, resolved_db)
        else:
            print("Pipeline (database): import-only mode")
            log.info(f"no database at {db_path} - skipping data-backed checks")
        print(log.render())
        log.lines.clear()

        print("Artifact write (temp dir):")
        _check_artifact_write(log)
        print(log.render())
    else:
        print(log.render())

    print("=" * 40)
    if log.ok:
        print("RESULT: PASS")
        if not resolved_db.exists():
            print("Notice: database absent - ran import-only checks.")
        return 0

    print("RESULT: FAIL")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
