"""Markdown report generation for Guardrail Alpha.

Standard-library only.
"""

from .attribution import regime_timeline, trade_attribution
from .db import event_counts, load_events
from .loaders import load_run_report
from .metrics import max_drawdown, nav_series, trade_count

AGENT_REPORT_EVENT = "agent_report_published"


def render_daily_report(summary: dict) -> str:
    """Render a minimal report from a pre-built summary dict."""
    return f"# Daily Report\n\n{summary}\n"


def _format_drawdown(raw: object) -> str:
    """Format a drawdown value (fraction string like "0.0432") as a percent."""
    try:
        return f"{float(raw) * 100:.2f}%"
    except (TypeError, ValueError):
        return "n/a"


def _format_nav(raw: object) -> str:
    """Format a NAV value (decimal string) as a USD amount."""
    try:
        return f"${float(raw):,.2f}"
    except (TypeError, ValueError):
        return "n/a"


def _top_positions_section(report: dict | None) -> list[str]:
    """Markdown lines for the top positions table, sorted by weight."""
    if not report:
        return ["_No position data available._"]

    positions = report.get("positions") or []
    if not positions:
        return ["_No open positions._"]

    def weight(position: dict) -> float:
        try:
            return float(position.get("weight_pct", 0.0))
        except (TypeError, ValueError):
            return 0.0

    ranked = sorted(positions, key=weight, reverse=True)[:10]

    lines = [
        "| Symbol | Value (USD) | Weight |",
        "| --- | ---: | ---: |",
    ]
    for position in ranked:
        symbol = position.get("symbol", "?")
        value = _format_nav(position.get("value_usd"))
        weight_pct = position.get("weight_pct", "0")
        try:
            weight_str = f"{float(weight_pct):.2f}%"
        except (TypeError, ValueError):
            weight_str = "n/a"
        lines.append(f"| {symbol} | {value} | {weight_str} |")
    return lines


def build_daily_report(
    db_path: str = "data/guardrail_alpha.db",
    report_path: str = "data/run_report.json",
) -> str:
    """Build a Markdown daily summary from the event log and run report."""
    events = load_events(db_path)
    report = load_run_report(report_path)

    counts = event_counts(events)
    trades = trade_count(events)
    series = nav_series(events)

    run_id = report.get("run_id", "unknown") if report else "unknown"
    mode = report.get("mode", "unknown") if report else "unknown"
    regime = report.get("regime", "unknown") if report else "unknown"
    kill_switch = report.get("kill_switch", False) if report else False

    if report and report.get("nav_usd") is not None:
        nav_display = _format_nav(report.get("nav_usd"))
    elif series:
        nav_display = _format_nav(series[-1][1])
    else:
        nav_display = "n/a"

    starting_nav = _format_nav(report.get("starting_nav_usd")) if report else "n/a"
    drawdown = _format_drawdown(report.get("total_drawdown_pct")) if report else "n/a"

    lines: list[str] = []
    lines.append("# Guardrail Alpha — Daily Report")
    lines.append("")
    lines.append("## Run")
    lines.append("")
    lines.append(f"- **Run ID:** `{run_id}`")
    lines.append(f"- **Mode:** {mode}")
    lines.append(f"- **Regime:** {regime}")
    lines.append(f"- **Kill switch:** {'TRIGGERED' if kill_switch else 'inactive'}")
    lines.append("")
    lines.append("## NAV & Risk")
    lines.append("")
    lines.append(f"- **Starting NAV:** {starting_nav}")
    lines.append(f"- **Current NAV:** {nav_display}")
    lines.append(f"- **Total drawdown:** {drawdown}")
    lines.append(f"- **Confirmed trades:** {trades}")
    lines.append(f"- **Reconciliation points:** {len(series)}")
    lines.append("")
    lines.append("## Top Positions")
    lines.append("")
    lines.extend(_top_positions_section(report))
    lines.append("")
    lines.append("## Event Counts")
    lines.append("")
    if counts:
        lines.append("| Event Type | Count |")
        lines.append("| --- | ---: |")
        for event_type, count in counts.items():
            lines.append(f"| {event_type} | {count} |")
    else:
        lines.append("_No events recorded._")
    lines.append("")

    return "\n".join(lines)


def _agent_report_payload(events: list[dict]) -> dict:
    """Payload of the latest ``agent_report_published`` event (or ``{}``)."""
    payload: dict = {}
    for event in events:
        if event.get("event_type") != AGENT_REPORT_EVENT:
            continue
        candidate = event.get("payload")
        if isinstance(candidate, dict):
            payload = candidate
    return payload


def _identity_value(*candidates: object, default: str = "unknown") -> str:
    """First non-empty candidate rendered as a string, else ``default``."""
    for candidate in candidates:
        if candidate is None:
            continue
        text = str(candidate).strip()
        if text:
            return text
    return default


def _attribution_section(attribution: list[dict]) -> list[str]:
    """Markdown lines for the trade-attribution table."""
    if not attribution:
        return ["_No confirmed swaps recorded._"]

    lines = [
        "| Destination | Confirmed Swaps | Total Amount (USD) |",
        "| --- | ---: | ---: |",
    ]
    for row in attribution:
        symbol = row.get("symbol", "?")
        count = row.get("count", 0)
        total = _format_nav(row.get("total_amount_usd"))
        lines.append(f"| {symbol} | {count} | {total} |")
    return lines


def _regime_section(timeline: list[dict]) -> list[str]:
    """Markdown lines summarizing the regime timeline."""
    if not timeline:
        return ["_No regime classifications recorded._"]

    counts: dict[str, int] = {}
    for entry in timeline:
        regime = entry.get("regime", "unknown")
        counts[regime] = counts.get(regime, 0) + 1

    lines = [
        f"- **Classifications:** {len(timeline)}",
        f"- **First:** `{timeline[0].get('regime', 'unknown')}` "
        f"@ {timeline[0].get('timestamp', '')}",
        f"- **Last:** `{timeline[-1].get('regime', 'unknown')}` "
        f"@ {timeline[-1].get('timestamp', '')}",
        "",
        "| Regime | Count |",
        "| --- | ---: |",
    ]
    for regime, count in sorted(counts.items()):
        lines.append(f"| {regime} | {count} |")
    return lines


def build_submission_report(
    db_path: str = "data/guardrail_alpha.db",
    report_path: str = "data/run_report.json",
) -> str:
    """Build a judge-facing Markdown submission report.

    Combines verifiable agent identity (id / wallet / policy hash / report
    hash, preferring the ``agent_report_published`` event and falling back to
    the run report), run statistics, a trade-attribution table, event counts,
    and a regime-timeline summary. Missing data degrades to ``unknown`` /
    placeholder rows rather than raising.
    """
    events = load_events(db_path)
    report = load_run_report(report_path) or {}
    agent = _agent_report_payload(events)

    counts = event_counts(events)
    trades = trade_count(events)
    series = nav_series(events)
    attribution = trade_attribution(events)
    timeline = regime_timeline(events)

    agent_id = _identity_value(agent.get("agent_id"), report.get("agent_id"))
    wallet = _identity_value(
        agent.get("wallet_address"),
        agent.get("wallet"),
        report.get("wallet_address"),
    )
    policy_hash = _identity_value(
        agent.get("policy_hash"), report.get("policy_hash")
    )
    report_hash = _identity_value(
        agent.get("report_hash"), report.get("report_hash")
    )
    run_id = _identity_value(agent.get("run_id"), report.get("run_id"))
    mode = _identity_value(report.get("mode"), agent.get("mode"))

    if agent.get("final_nav_usd") is not None:
        final_nav = _format_nav(agent.get("final_nav_usd"))
    elif report.get("nav_usd") is not None:
        final_nav = _format_nav(report.get("nav_usd"))
    elif series:
        final_nav = _format_nav(series[-1][1])
    else:
        final_nav = "n/a"

    starting_nav = _format_nav(report.get("starting_nav_usd"))
    drawdown = _format_drawdown(
        agent.get("total_drawdown_pct")
        if agent.get("total_drawdown_pct") is not None
        else report.get("total_drawdown_pct")
    )
    observed_drawdown = (
        _format_drawdown(max_drawdown([nav for _, nav in series]))
        if series
        else "n/a"
    )
    cycles = _identity_value(agent.get("cycles"), default="n/a")

    lines: list[str] = []
    lines.append("# Guardrail Alpha — Submission Report")
    lines.append("")
    lines.append("## Agent Identity")
    lines.append("")
    lines.append(f"- **Agent ID:** `{agent_id}`")
    lines.append(f"- **Wallet:** `{wallet}`")
    lines.append(f"- **Policy hash:** `{policy_hash}`")
    lines.append(f"- **Report hash:** `{report_hash}`")
    lines.append(f"- **Run ID:** `{run_id}`")
    lines.append(f"- **Mode:** {mode}")
    lines.append("")
    lines.append("## Run Statistics")
    lines.append("")
    lines.append(f"- **Cycles:** {cycles}")
    lines.append(f"- **Starting NAV:** {starting_nav}")
    lines.append(f"- **Final NAV:** {final_nav}")
    lines.append(f"- **Reported drawdown:** {drawdown}")
    lines.append(f"- **Observed max drawdown:** {observed_drawdown}")
    lines.append(f"- **Confirmed trades:** {trades}")
    lines.append(f"- **Reconciliation points:** {len(series)}")
    lines.append(f"- **Total events:** {len(events)}")
    lines.append("")
    lines.append("## Trade Attribution")
    lines.append("")
    lines.append("_Confirmed swaps grouped by destination symbol._")
    lines.append("")
    lines.extend(_attribution_section(attribution))
    lines.append("")
    lines.append("## Event Counts")
    lines.append("")
    if counts:
        lines.append("| Event Type | Count |")
        lines.append("| --- | ---: |")
        for event_type, count in counts.items():
            lines.append(f"| {event_type} | {count} |")
    else:
        lines.append("_No events recorded._")
    lines.append("")
    lines.append("## Regime Timeline")
    lines.append("")
    lines.extend(_regime_section(timeline))
    lines.append("")

    return "\n".join(lines)
