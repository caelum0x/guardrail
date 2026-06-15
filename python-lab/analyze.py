#!/usr/bin/env python3
"""Standalone analytics CLI for Guardrail Alpha.

Subcommands:

    regime      Regime-transition matrix, time-in-regime, exposure multipliers.
    drawdown    Underwater curve, max drawdown, and worst drawdown episodes.
    montecarlo  IID bootstrap risk simulation over the NAV curve (VaR/CVaR).
    dossier     One Markdown research dossier synthesizing every analytic.
    ensemble    Blended target book + attribution across the 4 Track-2 skills.
    ensemble-compare  Blend vs. each single skill: concentration / overlap.
    journal     Human-readable per-cycle decision journal from the event log.

Both subcommands load real data from the event-log database and the agent's
run report when those files exist, and print a readable text report. When no
data files are present they print a clear "no data" message and exit 0 (they
never crash on missing data).

Usage (from the repository root)::

    python3 python-lab/analyze.py regime
    python3 python-lab/analyze.py drawdown --db data/guardrail_alpha.db
    python3 python-lab/analyze.py dossier --out data/dossier.md

Standard-library only.
"""

from __future__ import annotations

import argparse
import sys
from datetime import timedelta
from pathlib import Path

# Make ``guardrail_lab`` importable whether this file is run as a script from
# the repo root (python3 python-lab/analyze.py) or from within python-lab.
_PACKAGE_ROOT = Path(__file__).resolve().parent
if str(_PACKAGE_ROOT) not in sys.path:
    sys.path.insert(0, str(_PACKAGE_ROOT))

from guardrail_lab import correlation as co  # noqa: E402
from guardrail_lab import dossier as ds  # noqa: E402
from guardrail_lab import dossier_html as dsh  # noqa: E402
from guardrail_lab import drawdown as dd  # noqa: E402
from guardrail_lab import ensemble as en  # noqa: E402
from guardrail_lab import ensemble_compare as ec  # noqa: E402
from guardrail_lab import journal as jn  # noqa: E402
from guardrail_lab import journal_html as jnh  # noqa: E402
from guardrail_lab import montecarlo as mc  # noqa: E402
from guardrail_lab import regime_analysis as ra  # noqa: E402
from guardrail_lab import report_bundle as rb  # noqa: E402
from guardrail_lab import seed as sd  # noqa: E402
from guardrail_lab.db import load_events  # noqa: E402
from guardrail_lab.metrics import nav_series  # noqa: E402

DEFAULT_DB = "data/guardrail_alpha.db"
DEFAULT_REPORT = "data/run_report.json"
DEFAULT_ENSEMBLE_CONFIG = "skills/ensemble.json"
DEFAULT_SKILLS_ROOT = "skills"
DEFAULT_SNAPSHOT_DIR = co.DEFAULT_SNAPSHOT_DIR
DEFAULT_REGIME = "risk_on"
DEFAULT_BUNDLE_OUT = rb.DEFAULT_OUT_DIR
NO_DATA_MESSAGE = (
    "no data — run the agent first "
    "(expected event log and/or run report under data/)."
)


def _format_seconds(seconds: float | None) -> str:
    """Render a duration in seconds as a compact human-readable string."""
    if seconds is None:
        return "n/a"
    if seconds <= 0:
        return "0s"
    return str(timedelta(seconds=round(seconds)))


def _print_header(title: str) -> None:
    """Print a section header underlined to its width."""
    print(title)
    print("=" * len(title))


def run_regime(db_path: str) -> int:
    """Load events and print the regime analytics report.

    Returns the process exit code (always ``0``: missing data is reported, not
    an error).
    """
    events = load_events(db_path)
    if not events:
        print(NO_DATA_MESSAGE)
        print(f"(looked for event log at: {db_path})")
        return 0

    analysis = ra.analyze_regimes(events)

    _print_header("Guardrail Alpha — Regime Analysis")
    print(f"Event log: {db_path}")
    print(f"Events: {len(events)}")
    print()

    print("Time in Regime")
    print("-" * 14)
    if analysis.time_in_regime:
        for entry in analysis.time_in_regime:
            print(
                f"  {entry.regime:<12} "
                f"classifications={entry.classifications:<4} "
                f"share={entry.fraction * 100:6.2f}%  "
                f"time={_format_seconds(entry.seconds)}"
            )
    else:
        print("  (no regime classifications recorded)")
    print()

    print("Transition Matrix (counts -> P)")
    print("-" * 31)
    matrix = analysis.transitions
    if matrix.total_transitions:
        for source in matrix.regimes:
            row = matrix.counts[source]
            successors = [
                f"{target}: {row[target]} "
                f"({matrix.probabilities[source][target] * 100:.0f}%)"
                for target in matrix.regimes
                if row[target] > 0
            ]
            if successors:
                print(f"  {source:<12} -> " + ", ".join(successors))
        print(f"  total transitions: {matrix.total_transitions}")
    else:
        print("  (need at least two classifications for transitions)")
    print()

    print("Average Exposure Multiplier per Regime")
    print("-" * 38)
    if analysis.exposure:
        for entry in analysis.exposure:
            print(
                f"  {entry.regime:<12} "
                f"orders={entry.order_count:<4} "
                f"avg_order=${entry.avg_order_usd:,.2f}  "
                f"multiplier={entry.exposure_multiplier:.2f}x"
            )
    else:
        print("  (no orders proposed)")
    print()

    return 0


def run_drawdown(db_path: str, top_n: int) -> int:
    """Load the NAV curve and print the drawdown report.

    Returns the process exit code (always ``0``).
    """
    events = load_events(db_path)
    curve = nav_series(events) if events else []
    if not curve:
        print(NO_DATA_MESSAGE)
        print(f"(looked for event log at: {db_path})")
        return 0

    report = dd.analyze_drawdown(curve, top_n=top_n)

    _print_header("Guardrail Alpha — Drawdown Analysis")
    print(f"Event log: {db_path}")
    print(f"NAV points: {len(curve)}")
    print(f"First NAV: ${curve[0][1]:,.2f}   Last NAV: ${curve[-1][1]:,.2f}")
    print()

    print("Max Drawdown")
    print("-" * 12)
    print(f"  depth:          {report.max_drawdown_pct:.4f}%")
    print(f"  peak:           {report.peak_timestamp or 'n/a'}")
    print(f"  trough:         {report.trough_timestamp or 'n/a'}")
    print(f"  duration:       {_format_seconds(report.max_drawdown_seconds)}")
    print(f"  recovery time:  {_format_seconds(report.max_recovery_seconds)}")
    print()

    print(f"Top {top_n} Worst Drawdown Episodes")
    print("-" * 30)
    if report.episodes:
        for index, episode in enumerate(report.episodes, start=1):
            status = "recovered" if episode.recovered else "UNRECOVERED"
            print(
                f"  {index}. {episode.depth_pct:.4f}%  "
                f"peak ${episode.peak_nav:,.2f} @ {episode.peak_timestamp} -> "
                f"trough ${episode.trough_nav:,.2f} @ "
                f"{episode.trough_timestamp}"
            )
            print(
                f"     duration={_format_seconds(episode.drawdown_seconds)}  "
                f"recovery={_format_seconds(episode.recovery_seconds)}  "
                f"[{status}]"
            )
    else:
        print("  (no drawdown episodes — NAV never declined from its peak)")
    print()

    return 0


def run_montecarlo(
    db_path: str,
    n_paths: int,
    seed: int,
    dd_threshold_pct: float,
) -> int:
    """Run an IID bootstrap risk simulation over the NAV curve and print it.

    Returns the process exit code (always ``0``: missing/short data is reported,
    not an error). ``dd_threshold_pct`` is given in percent (e.g. ``24`` for the
    kill-switch level) and converted to a fraction for the simulator.
    """
    events = load_events(db_path)
    curve = nav_series(events) if events else []
    if not curve:
        print(NO_DATA_MESSAGE)
        print(f"(looked for event log at: {db_path})")
        return 0

    threshold = max(0.0, dd_threshold_pct) / 100.0
    report = mc.bootstrap(
        curve, n_paths=n_paths, seed=seed, dd_threshold=threshold
    )

    _print_header("Guardrail Alpha — Monte Carlo Risk (IID bootstrap)")
    print(f"Event log: {db_path}")
    print(f"NAV points: {len(curve)}   Start NAV: ${report.start_nav:,.2f}")
    if not report.ok:
        print()
        print(f"  (not simulated: {report.reason})")
        print()
        return 0

    print(
        f"Paths: {report.n_paths}   Horizon: {report.horizon} steps   "
        f"Seed: {report.seed}   Returns sampled: {report.n_returns}"
    )
    print()

    print("Terminal Return Percentiles")
    print("-" * 27)
    for pct in (5.0, 25.0, 50.0, 75.0, 95.0):
        ret = report.terminal_return_percentiles.get(pct)
        nav = report.terminal_nav_percentiles.get(pct)
        if ret is not None:
            print(
                f"  p{int(pct):<3} return={ret * 100:7.2f}%   "
                f"NAV=${nav:,.2f}"
            )
    print(f"  mean return={report.terminal_return_mean * 100:7.2f}%")
    print()

    print("Tail Risk — Terminal Loss")
    print("-" * 25)
    for level in sorted(report.var_terminal):
        tail = report.var_terminal[level]
        print(
            f"  {int(level * 100)}%  VaR={tail.var * 100:6.2f}%   "
            f"CVaR={tail.cvar * 100:6.2f}%"
        )
    print()

    print("Tail Risk — Worst Drawdown")
    print("-" * 26)
    for level in sorted(report.var_drawdown):
        tail = report.var_drawdown[level]
        print(
            f"  {int(level * 100)}%  VaR={tail.var * 100:6.2f}%   "
            f"CVaR={tail.cvar * 100:6.2f}%"
        )
    print(
        f"  mean worst drawdown across paths="
        f"{report.worst_drawdowns_mean * 100:.2f}%"
    )
    print()

    print(
        f"P(drawdown breaches {dd_threshold_pct:g}%) = "
        f"{report.prob_breach * 100:.2f}%"
    )
    print()

    return 0


def run_dossier(
    db_path: str,
    report_path: str,
    out_path: str | None,
    as_html: bool,
) -> int:
    """Build the research dossier (Markdown or HTML) and print it to stdout.

    Always exits ``0``. When no data files exist the underlying builders
    return a clear "no data — run the agent first" skeleton rather than
    raising, so the command remains safe to run on a fresh checkout. When
    ``out_path`` is provided the dossier is also written to that file.

    HTML output is selected by either the ``as_html`` flag or an ``out_path``
    whose name ends in ``.html``/``.htm``; otherwise Markdown is emitted. In
    HTML mode the result is a single self-contained document (inline dark-theme
    CSS, no external CSS/JS/CDN references) produced by
    :func:`dsh.build_dossier_html`, which reuses :func:`ds.build_dossier`.
    """
    wants_html = as_html or (
        out_path is not None and out_path.lower().endswith((".html", ".htm"))
    )

    if wants_html:
        document = dsh.build_dossier_html(db_path, report_path)
        if out_path:
            dsh.write_dossier_html(
                out_path, db_path=db_path, report_path=report_path
            )
        print(document)
    else:
        markdown = ds.build_dossier(db_path, report_path)
        if out_path:
            ds.write_dossier(
                out_path, db_path=db_path, report_path=report_path
            )
        print(markdown)

    if out_path:
        print(f"\n(wrote dossier to: {out_path})", file=sys.stderr)
    return 0


def _latest_regime_from_db(db_path: str) -> str | None:
    """Return the most recent classified regime from the event log, if any.

    Reuses the existing event loader and regime sequence helper rather than
    re-querying the database. Returns ``None`` when no usable classification
    exists (missing DB, no ``regime_classified`` events, or unknown labels).
    """
    events = load_events(db_path)
    if not events:
        return None
    sequence = ra.regime_sequence(events)
    for _timestamp, regime in reversed(sequence):
        if regime and regime != ra.UNKNOWN_REGIME:
            return regime
    return None


def run_ensemble(
    regime: str | None,
    config_path: str,
    skills_root: str,
    db_path: str,
) -> int:
    """Blend the four Track-2 skills for a regime and print the result.

    When ``regime`` is ``None`` the current regime is taken from the event log
    if present, otherwise it falls back to :data:`DEFAULT_REGIME`. Always exits
    ``0``: a missing config or regime is reported via the rendered result, not
    raised.
    """
    resolved = regime
    source = "explicit --regime"
    if resolved is None:
        resolved = _latest_regime_from_db(db_path)
        if resolved is not None:
            source = f"current regime from {db_path}"
        else:
            resolved = DEFAULT_REGIME
            source = "default (no regime in data)"

    result = en.blend_regime(
        resolved, config_path=config_path, skills_root=skills_root
    )
    print(en.render_markdown(result))
    print(f"\n(regime source: {source})", file=sys.stderr)
    return 0


def run_ensemble_compare(
    regime: str | None,
    do_all: bool,
    config_path: str,
    skills_root: str,
    db_path: str,
) -> int:
    """Compare the blended ensemble book against each single skill, per regime.

    With ``do_all`` (``--all``) every known regime is compared and the sections
    are joined into one Markdown document. Otherwise a single regime is
    compared: an explicit ``--regime`` if given, else the current regime from
    the event log, else :data:`DEFAULT_REGIME`. Always exits ``0``: a missing
    config or absent skill examples are reported via the rendered output, not
    raised.
    """
    if do_all:
        comparisons = ec.compare_all(
            config_path=config_path, skills_root=skills_root
        )
        print(ec.render_markdown_all(comparisons))
        print("\n(compared all regimes)", file=sys.stderr)
        return 0

    resolved = regime
    source = "explicit --regime"
    if resolved is None:
        resolved = _latest_regime_from_db(db_path)
        if resolved is not None:
            source = f"current regime from {db_path}"
        else:
            resolved = DEFAULT_REGIME
            source = "default (no regime in data)"

    comparison = ec.compare_regime(
        resolved, config_path=config_path, skills_root=skills_root
    )
    print(ec.render_markdown(comparison))
    print(f"\n(regime source: {source})", file=sys.stderr)
    return 0


def run_journal(
    db_path: str,
    limit: int | None,
    out_path: str | None,
    as_html: bool,
) -> int:
    """Build the decision journal from the event log and print it.

    Always exits ``0``. When the event log is empty the rendered journal is a
    clear "no data" note rather than an error. ``limit`` caps the number of
    scored assets shown per cycle.

    Output format is Markdown by default. HTML output is selected by the
    ``as_html`` flag or an ``out_path`` whose name ends in ``.html``/``.htm``;
    in HTML mode the result is a single self-contained document (inline
    dark-theme CSS, no external CSS/JS/CDN references) produced by
    :func:`jnh.build_journal_html`, which reuses :func:`jn.render_markdown` and
    the shared Markdown->HTML converter. When ``out_path`` is given the rendered
    document is also written to that file.
    """
    top_n = limit if (limit is not None and limit > 0) else 5
    wants_html = as_html or (
        out_path is not None and out_path.lower().endswith((".html", ".htm"))
    )

    if wants_html:
        document = jnh.build_journal_html(db_path=db_path, top_n=top_n)
        print(document)
        if out_path:
            jnh.write_journal_html(out_path, db_path=db_path, top_n=top_n)
            print(f"\n(wrote journal to: {out_path})", file=sys.stderr)
        return 0

    journal = jn.build_journal_from_db(db_path)
    markdown = jn.render_markdown(journal, top_n=top_n)
    print(markdown)

    if out_path:
        try:
            Path(out_path).write_text(markdown, encoding="utf-8")
            print(f"\n(wrote journal to: {out_path})", file=sys.stderr)
        except OSError as error:
            print(f"\n(could not write journal: {error})", file=sys.stderr)
    return 0


def run_bundle(
    out_dir: str,
    db_path: str,
    report_path: str,
    config_path: str,
    skills_root: str,
) -> int:
    """Build the folder of self-contained HTML reports and print the index path.

    Reuses :func:`guardrail_lab.report_bundle.build_bundle`, which composes the
    existing dossier / journal / ensemble-comparison renderers into one
    browsable bundle (``index.html`` linking ``dossier.html``, ``journal.html``,
    and ``ensemble.html``). Always exits ``0``: on missing data each underlying
    builder writes a valid no-data skeleton rather than raising, so a fresh
    checkout still produces a complete, well-formed bundle.

    Args:
        out_dir: Destination directory for the bundle.
        db_path: Path to the SQLite event-log database.
        report_path: Path to the agent's JSON run report.
        config_path: Path to the ensemble blend-weights config.
        skills_root: Root directory holding the skill directories.

    Returns:
        The process exit code (always ``0``).
    """
    written = rb.build_bundle(
        out_dir=out_dir,
        db_path=db_path,
        report_path=report_path,
        config_path=config_path,
        skills_root=skills_root,
    )
    index_path = next(
        (path for path in written if path.endswith(rb.INDEX_FILE)),
        written[0] if written else out_dir,
    )
    print(index_path)
    print(f"\n(wrote {len(written)} report(s) to: {out_dir})", file=sys.stderr)
    return 0


def run_seed(
    db_path: str,
    report_path: str,
    cycles: int,
    seed: int,
) -> int:
    """Generate a deterministic synthetic run into the demo files and summarize.

    Delegates to :func:`guardrail_lab.seed.seed_demo`, which writes a
    multi-regime, multi-cycle event-log database and a matching run report to a
    SEPARATE demo location (defaults under ``data/demo_*``) so the analytics
    demo richly without touching the real ``data/guardrail_alpha.db`` /
    ``data/run_report.json``. Prints a summary (events written, regimes
    covered, NAV range) and returns ``0`` on success.
    """
    result = sd.seed_demo(
        db_path=db_path,
        report_path=report_path,
        cycles=cycles,
        seed=seed,
    )

    _print_header("Guardrail Alpha — Demo Seeder")
    print(f"Database:   {result.db_path}")
    print(f"Run report: {result.report_path}")
    print(f"Run ID:     {result.run_id}")
    print(f"Seed:       {seed}")
    print()
    print(f"Cycles:         {result.cycles}")
    print(f"Events written: {result.events_written}")
    print(f"Confirmed trades: {result.trades}")
    print(f"Regimes covered: {', '.join(result.regimes)}")
    print()
    print(
        f"NAV range: ${result.nav_min:,.2f} -> ${result.nav_max:,.2f}  "
        f"(final ${result.final_nav:,.2f})"
    )
    print(f"Max drawdown: {result.max_drawdown_pct:.4f}%")
    print()
    print("Try the analytics against the demo database, e.g.:")
    print(f"  python3 python-lab/analyze.py regime --db {result.db_path}")
    print(f"  python3 python-lab/analyze.py drawdown --db {result.db_path}")
    print(
        f"  python3 python-lab/analyze.py dossier "
        f"--db {result.db_path} --report {result.report_path}"
    )
    print()
    return 0


def run_correlation(
    snapshot_dir: str,
    report_path: str,
    db_path: str,
    top_n: int,
) -> int:
    """Print the asset correlation matrix and exposure/concentration summary.

    Loads the price/return history (snapshots preferred, event log fallback) and
    the latest target book (run report), then prints the most-correlated asset
    pairs, a compact correlation matrix, and the concentration diagnostics.
    Always exits ``0``: missing/insufficient data is reported via a clear
    message rather than raised.
    """
    report = co.analyze_correlation(
        snapshot_dir=snapshot_dir,
        report_path=report_path,
        db_path=db_path,
    )

    _print_header("Guardrail Alpha — Correlation & Exposure")
    print(f"Snapshots: {snapshot_dir}   Run report: {report_path}")
    print(f"Price source: {report.source}")
    print()

    if not report.ok:
        print(report.reason)
        print()
        return 0

    print(f"({report.reason})")
    print()

    matrix = report.correlation
    show_n = top_n if top_n > 0 else 10

    print(f"Top {show_n} Correlated Pairs")
    print("-" * 24)
    defined_pairs = [pair for pair in matrix.pairs if pair.defined]
    if defined_pairs:
        for pair in defined_pairs[:show_n]:
            print(
                f"  {pair.a:<6} ~ {pair.b:<6} "
                f"corr={pair.correlation:+.4f}  "
                f"(n={pair.observations})"
            )
    else:
        print("  (no asset pair had enough overlapping returns)")
    print()

    print("Correlation Matrix")
    print("-" * 18)
    if matrix.symbols:
        header = "        " + "".join(f"{sym:>8}" for sym in matrix.symbols)
        print(header)
        for row_sym in matrix.symbols:
            cells = "".join(
                f"{matrix.matrix[row_sym][col_sym]:>8.2f}"
                for col_sym in matrix.symbols
            )
            print(f"  {row_sym:<6}{cells}")
        print(f"  assets={len(matrix.symbols)}  return steps={matrix.n_observations}")
    else:
        print("  (no correlation matrix — insufficient price history)")
    print()

    exposure = report.exposure
    print("Exposure & Concentration (latest target book)")
    print("-" * 45)
    if exposure.position_count:
        for position in exposure.positions:
            print(
                f"  {position.symbol:<6} "
                f"weight={position.weight_pct:6.2f}%"
            )
        print()
        print(f"  positions:            {exposure.position_count}")
        print(f"  gross weight:         {exposure.gross_weight * 100:.2f}%")
        print(f"  Herfindahl index:     {exposure.herfindahl:.4f}")
        print(
            f"  effective positions:  {exposure.effective_positions:.2f}"
        )
    else:
        print("  (no target book positions found)")
    print()

    return 0


def build_parser() -> argparse.ArgumentParser:
    """Construct the argparse CLI parser."""
    parser = argparse.ArgumentParser(
        prog="analyze.py",
        description="Guardrail Alpha analytics (regime + drawdown).",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    regime = subparsers.add_parser(
        "regime", help="Regime transition / time / exposure analytics."
    )
    regime.add_argument(
        "--db",
        default=DEFAULT_DB,
        help=f"Path to the event-log database (default: {DEFAULT_DB}).",
    )
    regime.add_argument(
        "--report",
        default=DEFAULT_REPORT,
        help=f"Path to the run report JSON (default: {DEFAULT_REPORT}).",
    )

    draw = subparsers.add_parser(
        "drawdown", help="NAV underwater curve and worst drawdown episodes."
    )
    draw.add_argument(
        "--db",
        default=DEFAULT_DB,
        help=f"Path to the event-log database (default: {DEFAULT_DB}).",
    )
    draw.add_argument(
        "--report",
        default=DEFAULT_REPORT,
        help=f"Path to the run report JSON (default: {DEFAULT_REPORT}).",
    )
    draw.add_argument(
        "--top-n",
        type=int,
        default=5,
        help="Number of worst drawdown episodes to show (default: 5).",
    )

    montecarlo = subparsers.add_parser(
        "montecarlo",
        help="IID bootstrap risk simulation over the NAV curve (VaR/CVaR).",
    )
    montecarlo.add_argument(
        "--db",
        default=DEFAULT_DB,
        help=f"Path to the event-log database (default: {DEFAULT_DB}).",
    )
    montecarlo.add_argument(
        "--report",
        default=DEFAULT_REPORT,
        help=f"Path to the run report JSON (default: {DEFAULT_REPORT}).",
    )
    montecarlo.add_argument(
        "--paths",
        type=int,
        default=mc.DEFAULT_N_PATHS,
        help=f"Number of bootstrap paths (default: {mc.DEFAULT_N_PATHS}).",
    )
    montecarlo.add_argument(
        "--seed",
        type=int,
        default=mc.DEFAULT_SEED,
        help=f"RNG seed for reproducibility (default: {mc.DEFAULT_SEED}).",
    )
    montecarlo.add_argument(
        "--dd-threshold",
        type=float,
        default=mc.DEFAULT_DD_THRESHOLD * 100.0,
        help=(
            "Drawdown breach threshold in percent "
            f"(default: {mc.DEFAULT_DD_THRESHOLD * 100.0:g})."
        ),
    )

    dossier = subparsers.add_parser(
        "dossier",
        help="Synthesize every analytic into one Markdown research dossier.",
    )
    dossier.add_argument(
        "--db",
        default=DEFAULT_DB,
        help=f"Path to the event-log database (default: {DEFAULT_DB}).",
    )
    dossier.add_argument(
        "--report",
        default=DEFAULT_REPORT,
        help=f"Path to the run report JSON (default: {DEFAULT_REPORT}).",
    )
    dossier.add_argument(
        "--out",
        default=None,
        help=(
            "Optional path to also write the dossier to. A .html/.htm "
            "suffix selects HTML output automatically."
        ),
    )
    dossier.add_argument(
        "--html",
        action="store_true",
        help="Emit a self-contained HTML document instead of Markdown.",
    )

    ensemble = subparsers.add_parser(
        "ensemble",
        help="Blend the 4 Track-2 skills by regime (book + attribution).",
    )
    ensemble.add_argument(
        "--regime",
        default=None,
        help=(
            "Regime to blend (risk_on/risk_off/chop/breakout). Defaults to the "
            f"current regime in the event log, else {DEFAULT_REGIME}."
        ),
    )
    ensemble.add_argument(
        "--config",
        default=DEFAULT_ENSEMBLE_CONFIG,
        help=(
            "Path to the ensemble blend-weights config "
            f"(default: {DEFAULT_ENSEMBLE_CONFIG})."
        ),
    )
    ensemble.add_argument(
        "--skills-root",
        default=DEFAULT_SKILLS_ROOT,
        help=(
            "Root directory holding the 4 skill directories "
            f"(default: {DEFAULT_SKILLS_ROOT})."
        ),
    )
    ensemble.add_argument(
        "--db",
        default=DEFAULT_DB,
        help=f"Path to the event-log database (default: {DEFAULT_DB}).",
    )

    compare = subparsers.add_parser(
        "ensemble-compare",
        help=(
            "Compare the blended ensemble book vs. each single skill "
            "(concentration / diversification / overlap) by regime."
        ),
    )
    compare.add_argument(
        "--regime",
        default=None,
        help=(
            "Regime to compare (risk_on/risk_off/chop/breakout). Defaults to "
            f"the current regime in the event log, else {DEFAULT_REGIME}. "
            "Ignored when --all is given."
        ),
    )
    compare.add_argument(
        "--all",
        action="store_true",
        help="Compare all four regimes and emit one combined report.",
    )
    compare.add_argument(
        "--config",
        default=DEFAULT_ENSEMBLE_CONFIG,
        help=(
            "Path to the ensemble blend-weights config "
            f"(default: {DEFAULT_ENSEMBLE_CONFIG})."
        ),
    )
    compare.add_argument(
        "--skills-root",
        default=DEFAULT_SKILLS_ROOT,
        help=(
            "Root directory holding the 4 skill directories "
            f"(default: {DEFAULT_SKILLS_ROOT})."
        ),
    )
    compare.add_argument(
        "--db",
        default=DEFAULT_DB,
        help=f"Path to the event-log database (default: {DEFAULT_DB}).",
    )

    journal = subparsers.add_parser(
        "journal",
        help="Human-readable per-cycle decision journal from the event log.",
    )
    journal.add_argument(
        "--db",
        default=DEFAULT_DB,
        help=f"Path to the event-log database (default: {DEFAULT_DB}).",
    )
    journal.add_argument(
        "--limit",
        type=int,
        default=5,
        help="Max scored assets to list per cycle (default: 5).",
    )
    journal.add_argument(
        "--out",
        default=None,
        help=(
            "Optional path to also write the journal to. A .html/.htm suffix "
            "selects HTML output automatically."
        ),
    )
    journal.add_argument(
        "--html",
        action="store_true",
        help=(
            "Emit a self-contained HTML document (inline dark-theme CSS, no "
            "external CSS/JS/CDN) instead of Markdown."
        ),
    )

    correlation = subparsers.add_parser(
        "correlation",
        help=(
            "Per-asset return correlation matrix (Pearson) plus "
            "concentration / exposure summary (HHI, effective positions)."
        ),
    )
    correlation.add_argument(
        "--snapshots",
        default=DEFAULT_SNAPSHOT_DIR,
        help=(
            "Directory of market snapshot .jsonl files "
            f"(default: {DEFAULT_SNAPSHOT_DIR})."
        ),
    )
    correlation.add_argument(
        "--report",
        default=DEFAULT_REPORT,
        help=(
            "Path to the run report JSON (latest target book) "
            f"(default: {DEFAULT_REPORT})."
        ),
    )
    correlation.add_argument(
        "--db",
        default=DEFAULT_DB,
        help=(
            "Path to the event-log database (fallback price source) "
            f"(default: {DEFAULT_DB})."
        ),
    )
    correlation.add_argument(
        "--top-n",
        type=int,
        default=10,
        help="Number of top correlated pairs to show (default: 10).",
    )

    bundle = subparsers.add_parser(
        "bundle",
        help=(
            "Write a folder of self-contained HTML reports (index + dossier + "
            "journal + ensemble) and print the index path."
        ),
    )
    bundle.add_argument(
        "--out",
        default=DEFAULT_BUNDLE_OUT,
        help=(
            "Output directory for the report bundle "
            f"(default: {DEFAULT_BUNDLE_OUT})."
        ),
    )
    bundle.add_argument(
        "--db",
        default=DEFAULT_DB,
        help=f"Path to the event-log database (default: {DEFAULT_DB}).",
    )
    bundle.add_argument(
        "--report",
        default=DEFAULT_REPORT,
        help=f"Path to the run report JSON (default: {DEFAULT_REPORT}).",
    )
    bundle.add_argument(
        "--config",
        default=DEFAULT_ENSEMBLE_CONFIG,
        help=(
            "Path to the ensemble blend-weights config "
            f"(default: {DEFAULT_ENSEMBLE_CONFIG})."
        ),
    )
    bundle.add_argument(
        "--skills-root",
        default=DEFAULT_SKILLS_ROOT,
        help=(
            "Root directory holding the 4 skill directories "
            f"(default: {DEFAULT_SKILLS_ROOT})."
        ),
    )

    seed = subparsers.add_parser(
        "seed",
        help=(
            "Generate a deterministic multi-regime, multi-cycle synthetic run "
            "into a SEPARATE demo database + run report (real data untouched)."
        ),
    )
    seed.add_argument(
        "--db",
        default=sd.DEFAULT_DB,
        help=(
            "Destination demo event-log database "
            f"(default: {sd.DEFAULT_DB})."
        ),
    )
    seed.add_argument(
        "--report",
        default=sd.DEFAULT_REPORT,
        help=(
            "Destination demo run report JSON "
            f"(default: {sd.DEFAULT_REPORT})."
        ),
    )
    seed.add_argument(
        "--cycles",
        type=int,
        default=sd.DEFAULT_CYCLES,
        help=(
            "Approximate number of decision cycles to generate "
            f"(default: {sd.DEFAULT_CYCLES})."
        ),
    )
    seed.add_argument(
        "--seed",
        type=int,
        default=sd.DEFAULT_SEED,
        help=(
            "Integer RNG seed for deterministic output "
            f"(default: {sd.DEFAULT_SEED})."
        ),
    )

    return parser


def main(argv: list[str] | None = None) -> int:
    """CLI entry point. Returns the process exit code."""
    parser = build_parser()
    args = parser.parse_args(argv)

    if args.command == "regime":
        return run_regime(args.db)
    if args.command == "drawdown":
        return run_drawdown(args.db, args.top_n)
    if args.command == "montecarlo":
        return run_montecarlo(
            args.db, args.paths, args.seed, args.dd_threshold
        )
    if args.command == "dossier":
        return run_dossier(args.db, args.report, args.out, args.html)
    if args.command == "ensemble":
        return run_ensemble(
            args.regime, args.config, args.skills_root, args.db
        )
    if args.command == "ensemble-compare":
        return run_ensemble_compare(
            args.regime, args.all, args.config, args.skills_root, args.db
        )
    if args.command == "journal":
        return run_journal(args.db, args.limit, args.out, args.html)
    if args.command == "correlation":
        return run_correlation(
            args.snapshots, args.report, args.db, args.top_n
        )
    if args.command == "bundle":
        return run_bundle(
            args.out, args.db, args.report, args.config, args.skills_root
        )
    if args.command == "seed":
        return run_seed(args.db, args.report, args.cycles, args.seed)

    parser.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
