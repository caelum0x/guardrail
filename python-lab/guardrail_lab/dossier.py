"""Research-dossier generator for Guardrail Alpha.

Synthesizes every existing analytic into one cohesive Markdown research
dossier. This module is a *composition* layer: it reuses the established
loaders and analytics rather than re-implementing any metric:

* run metadata / summary from :mod:`guardrail_lab.loaders` and
  :func:`guardrail_lab.db.event_counts`,
* headline performance metrics from :mod:`guardrail_lab.metrics`,
* per-asset attribution from :mod:`guardrail_lab.attribution`,
* regime transition / time-in-regime / exposure from
  :mod:`guardrail_lab.regime_analysis`, and
* drawdown analysis (incl. worst episodes) from
  :mod:`guardrail_lab.drawdown`.

Design contract:

* :func:`build_dossier` is pure-ish — it reads the database / report files
  once and returns a Markdown string. It NEVER raises on missing or partial
  data; each section degrades to an explicit "no data" note instead.
* Standard-library only, so it runs without any pip installs.
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

from . import drawdown as dd
from . import regime_analysis as ra
from .attribution import trade_attribution
from .db import event_counts, load_events
from .loaders import load_run_report
from .metrics import max_drawdown, nav_series, trade_count

DEFAULT_DB = "data/guardrail_alpha.db"
DEFAULT_REPORT = "data/run_report.json"

#: Number of worst drawdown episodes to surface in the dossier.
WORST_EPISODE_LIMIT = 5


def _format_seconds(seconds: float | None) -> str:
    """Render a duration in seconds as a compact human-readable string."""
    if seconds is None:
        return "n/a"
    if seconds <= 0:
        return "0s"
    return str(timedelta(seconds=round(seconds)))


def _format_usd(raw: object) -> str:
    """Format a numeric (or decimal-string) value as a USD amount."""
    try:
        return f"${float(raw):,.2f}"  # type: ignore[arg-type]
    except (TypeError, ValueError):
        return "n/a"


def _format_fraction_pct(raw: object) -> str:
    """Format a fraction (e.g. ``0.0432``) as a percentage string."""
    try:
        return f"{float(raw) * 100:.2f}%"  # type: ignore[arg-type]
    except (TypeError, ValueError):
        return "n/a"


def _format_pct(raw: object) -> str:
    """Format an already-percentage value (e.g. ``-4.32``) as a percent."""
    try:
        return f"{float(raw):.4f}%"  # type: ignore[arg-type]
    except (TypeError, ValueError):
        return "n/a"


def _identity(*candidates: object, default: str = "unknown") -> str:
    """First non-empty candidate rendered as a string, else ``default``."""
    for candidate in candidates:
        if candidate is None:
            continue
        text = str(candidate).strip()
        if text:
            return text
    return default


def _no_data_note(reason: str) -> list[str]:
    """Standardized italic note line for an empty section."""
    return [f"_No data for this section ({reason})._"]


def _metadata_section(report: dict | None, events: list[dict]) -> list[str]:
    """Markdown lines for the run summary / metadata section.

    Prefers fields from the run report and falls back to the event log so the
    section still renders useful context when one source is absent.
    """
    if not report and not events:
        return _no_data_note("no run report and no events")

    report = report or {}
    run_id = _identity(report.get("run_id"))
    mode = _identity(report.get("mode"))
    regime = _identity(report.get("regime"))
    agent_id = _identity(report.get("agent_id"))
    kill_switch = bool(report.get("kill_switch", False))

    first_ts = events[0].get("timestamp") if events else None
    last_ts = events[-1].get("timestamp") if events else None

    lines = [
        f"- **Run ID:** `{run_id}`",
        f"- **Agent ID:** `{agent_id}`",
        f"- **Mode:** {mode}",
        f"- **Declared regime:** {regime}",
        f"- **Kill switch:** {'TRIGGERED' if kill_switch else 'inactive'}",
        f"- **Total events:** {len(events)}",
        f"- **First event:** {_identity(first_ts, default='n/a')}",
        f"- **Last event:** {_identity(last_ts, default='n/a')}",
    ]
    return lines


def _performance_section(
    report: dict | None, events: list[dict]
) -> list[str]:
    """Markdown lines for the headline performance metrics section."""
    series = nav_series(events)
    trades = trade_count(events)

    if not series and not report:
        return _no_data_note("no NAV history and no run report")

    report = report or {}

    if report.get("nav_usd") is not None:
        current_nav = _format_usd(report.get("nav_usd"))
    elif series:
        current_nav = _format_usd(series[-1][1])
    else:
        current_nav = "n/a"

    starting_nav = _format_usd(report.get("starting_nav_usd"))
    reported_dd = _format_fraction_pct(report.get("total_drawdown_pct"))
    observed_dd = (
        _format_fraction_pct(max_drawdown([nav for _, nav in series]))
        if series
        else "n/a"
    )

    lines = [
        f"- **Starting NAV:** {starting_nav}",
        f"- **Current NAV:** {current_nav}",
        f"- **Reported drawdown:** {reported_dd}",
        f"- **Observed max drawdown:** {observed_dd}",
        f"- **Confirmed trades:** {trades}",
        f"- **Reconciliation points:** {len(series)}",
    ]

    counts = event_counts(events)
    if counts:
        lines.append("")
        lines.append("| Event Type | Count |")
        lines.append("| --- | ---: |")
        for event_type, count in counts.items():
            lines.append(f"| {event_type} | {count} |")
    return lines


def _attribution_section(events: list[dict]) -> list[str]:
    """Markdown lines for the per-asset trade-attribution section."""
    attribution = trade_attribution(events)
    if not attribution:
        return _no_data_note("no confirmed swaps recorded")

    lines = [
        "_Confirmed swaps grouped by destination symbol._",
        "",
        "| Symbol | Confirmed Swaps | Total Amount (USD) |",
        "| --- | ---: | ---: |",
    ]
    for row in attribution:
        symbol = _identity(row.get("symbol"), default="?")
        count = row.get("count", 0)
        total = _format_usd(row.get("total_amount_usd"))
        lines.append(f"| {symbol} | {count} | {total} |")
    return lines


def _regime_section(events: list[dict]) -> list[str]:
    """Markdown lines for the regime transition / time / exposure section."""
    analysis = ra.analyze_regimes(events)
    has_data = bool(
        analysis.time_in_regime
        or analysis.exposure
        or analysis.transitions.total_transitions
    )
    if not has_data:
        return _no_data_note("no regime classifications recorded")

    lines: list[str] = []

    lines.append("### Time in Regime")
    lines.append("")
    if analysis.time_in_regime:
        lines.append("| Regime | Classifications | Share | Wall-clock Time |")
        lines.append("| --- | ---: | ---: | ---: |")
        for entry in analysis.time_in_regime:
            lines.append(
                f"| {entry.regime} | {entry.classifications} | "
                f"{entry.fraction * 100:.2f}% | "
                f"{_format_seconds(entry.seconds)} |"
            )
    else:
        lines.append("_No regime classifications recorded._")
    lines.append("")

    lines.append("### Regime Transitions")
    lines.append("")
    matrix = analysis.transitions
    if matrix.total_transitions:
        for source in matrix.regimes:
            row = matrix.counts[source]
            successors = [
                f"`{target}` {row[target]} "
                f"({matrix.probabilities[source][target] * 100:.0f}%)"
                for target in matrix.regimes
                if row[target] > 0
            ]
            if successors:
                lines.append(f"- `{source}` -> " + ", ".join(successors))
        lines.append(f"- **Total transitions:** {matrix.total_transitions}")
    else:
        lines.append(
            "_Need at least two classifications for transitions._"
        )
    lines.append("")

    lines.append("### Average Exposure Multiplier per Regime")
    lines.append("")
    if analysis.exposure:
        lines.append("| Regime | Orders | Avg Order (USD) | Multiplier |")
        lines.append("| --- | ---: | ---: | ---: |")
        for entry in analysis.exposure:
            lines.append(
                f"| {entry.regime} | {entry.order_count} | "
                f"{_format_usd(entry.avg_order_usd)} | "
                f"{entry.exposure_multiplier:.2f}x |"
            )
    else:
        lines.append("_No orders proposed._")

    return lines


def _drawdown_section(events: list[dict]) -> list[str]:
    """Markdown lines for the drawdown analysis (incl. worst episodes)."""
    curve = nav_series(events)
    if not curve:
        return _no_data_note("no NAV history to compute drawdowns")

    report = dd.analyze_drawdown(curve, top_n=WORST_EPISODE_LIMIT)

    lines = [
        f"- **NAV points:** {len(curve)}",
        f"- **First NAV:** {_format_usd(curve[0][1])}",
        f"- **Last NAV:** {_format_usd(curve[-1][1])}",
        "",
        "### Max Drawdown",
        "",
        f"- **Depth:** {_format_pct(report.max_drawdown_pct)}",
        f"- **Peak:** {_identity(report.peak_timestamp, default='n/a')}",
        f"- **Trough:** {_identity(report.trough_timestamp, default='n/a')}",
        f"- **Duration:** {_format_seconds(report.max_drawdown_seconds)}",
        f"- **Recovery time:** "
        f"{_format_seconds(report.max_recovery_seconds)}",
        "",
        f"### Top {WORST_EPISODE_LIMIT} Worst Drawdown Episodes",
        "",
    ]

    if report.episodes:
        lines.append(
            "| # | Depth | Peak | Trough | Duration | Recovery | Status |"
        )
        lines.append("| ---: | ---: | --- | --- | ---: | ---: | --- |")
        for index, episode in enumerate(report.episodes, start=1):
            status = "recovered" if episode.recovered else "UNRECOVERED"
            lines.append(
                f"| {index} | {_format_pct(episode.depth_pct)} | "
                f"{_format_usd(episode.peak_nav)} @ "
                f"{episode.peak_timestamp} | "
                f"{_format_usd(episode.trough_nav)} @ "
                f"{episode.trough_timestamp} | "
                f"{_format_seconds(episode.drawdown_seconds)} | "
                f"{_format_seconds(episode.recovery_seconds)} | "
                f"{status} |"
            )
    else:
        lines.append(
            "_No drawdown episodes — NAV never declined from its peak._"
        )

    return lines


def _section(title: str, body: list[str]) -> list[str]:
    """Wrap a section body under a level-2 Markdown heading."""
    return [f"## {title}", "", *body, ""]


def build_dossier(
    db_path: str = DEFAULT_DB,
    report_path: str = DEFAULT_REPORT,
) -> str:
    """Build a full Markdown research dossier from whatever data exists.

    Loads the event log and run report once, then synthesizes the existing
    analytics into a single Markdown document combining:

    * run summary / metadata,
    * headline performance metrics,
    * per-asset attribution,
    * regime transition + time-in-regime + exposure, and
    * drawdown analysis including the worst episodes.

    Every section degrades gracefully: if its inputs are missing it renders a
    "no data for this section" note instead of raising. The function therefore
    always returns a valid Markdown string and never propagates exceptions
    from the underlying analytics.

    Args:
        db_path: Path to the SQLite event-log database.
        report_path: Path to the agent's JSON run report.

    Returns:
        The dossier rendered as a Markdown string.
    """
    events = load_events(db_path)
    report = load_run_report(report_path)

    generated_at = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    lines: list[str] = [
        "# Guardrail Alpha — Research Dossier",
        "",
        f"_Generated {generated_at} · sources: `{db_path}`, "
        f"`{report_path}`._",
        "",
    ]

    if not events and not report:
        lines.extend(
            _section(
                "Status",
                [
                    "_No data — run the agent first. Expected an event log "
                    f"at `{db_path}` and/or a run report at `{report_path}`._",
                ],
            )
        )
        return "\n".join(lines).rstrip() + "\n"

    lines.extend(_section("Run Summary", _metadata_section(report, events)))
    lines.extend(
        _section("Headline Performance", _performance_section(report, events))
    )
    lines.extend(
        _section("Per-Asset Attribution", _attribution_section(events))
    )
    lines.extend(_section("Regime Analysis", _regime_section(events)))
    lines.extend(_section("Drawdown Analysis", _drawdown_section(events)))

    return "\n".join(lines).rstrip() + "\n"


def write_dossier(
    path: str,
    db_path: str = DEFAULT_DB,
    report_path: str = DEFAULT_REPORT,
) -> str:
    """Build the dossier and write it to ``path``; return the Markdown.

    Args:
        path: Destination file path for the Markdown dossier.
        db_path: Path to the SQLite event-log database.
        report_path: Path to the agent's JSON run report.

    Returns:
        The dossier Markdown string that was written.
    """
    markdown = build_dossier(db_path=db_path, report_path=report_path)
    from pathlib import Path

    destination = Path(path)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(markdown, encoding="utf-8")
    return markdown
