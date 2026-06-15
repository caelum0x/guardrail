"""Self-contained HTML submission-report generator for Guardrail Alpha.

Produces a single, dependency-free HTML document summarizing a run for judges:
a header (project / run id / mode), key metrics (NAV, drawdown, regime, kill
switch), verifiable agent identity (wallet / policy hash / report hash / agent
id), the positions table, event-type counts, a trade-attribution table, and an
experiment-comparison table.

Standard-library only (json, pathlib, html, datetime). All dynamic text is
escaped with :func:`html.escape`, and every section degrades gracefully when its
underlying data is missing rather than raising.
"""

import html
from datetime import datetime, timezone

from .attribution import regime_timeline, trade_attribution
from .db import event_counts, load_events
from .experiments import compare_table, load_experiments
from .loaders import load_run_report
from .metrics import max_drawdown, nav_series, trade_count

AGENT_REPORT_EVENT = "agent_report_published"

PLACEHOLDER = "n/a"
UNKNOWN = "unknown"


# ---------------------------------------------------------------------------
# Value helpers (formatting + safe coercion). None of these raise.
# ---------------------------------------------------------------------------


def _esc(value: object) -> str:
    """HTML-escape any value, rendering ``None`` as an empty string."""
    if value is None:
        return ""
    return html.escape(str(value), quote=True)


def _identity_value(*candidates: object, default: str = UNKNOWN) -> str:
    """First non-empty candidate rendered as a string, else ``default``."""
    for candidate in candidates:
        if candidate is None:
            continue
        text = str(candidate).strip()
        if text:
            return text
    return default


def _format_nav(raw: object) -> str:
    """Format a NAV/amount value (number or decimal string) as USD."""
    try:
        return f"${float(raw):,.2f}"
    except (TypeError, ValueError):
        return PLACEHOLDER


def _format_drawdown_fraction(raw: object) -> str:
    """Format a drawdown fraction (e.g. "0.1025") as a percentage string."""
    try:
        return f"{float(raw) * 100:.2f}%"
    except (TypeError, ValueError):
        return PLACEHOLDER


def _format_pct_value(raw: object) -> str:
    """Format a value that is already a percentage (e.g. 30.841)."""
    try:
        return f"{float(raw):.2f}%"
    except (TypeError, ValueError):
        return PLACEHOLDER


def _format_number(raw: object, digits: int = 3) -> str:
    """Format a generic numeric value, blanking when not numeric."""
    try:
        return f"{float(raw):.{digits}f}"
    except (TypeError, ValueError):
        return PLACEHOLDER


def _format_int(raw: object) -> str:
    """Format an integer-ish value, blanking when not numeric."""
    try:
        return f"{int(float(raw)):,}"
    except (TypeError, ValueError):
        return PLACEHOLDER


# ---------------------------------------------------------------------------
# Data gathering
# ---------------------------------------------------------------------------


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


def _position_weight(position: dict) -> float:
    try:
        return float(position.get("weight_pct", 0.0))
    except (TypeError, ValueError):
        return 0.0


# ---------------------------------------------------------------------------
# HTML section renderers. Each returns an HTML fragment string.
# ---------------------------------------------------------------------------


def _render_metric_card(label: str, value: str, accent: str = "") -> str:
    accent_class = f" metric--{accent}" if accent else ""
    return (
        f'<div class="metric{accent_class}">'
        f'<div class="metric__label">{_esc(label)}</div>'
        f'<div class="metric__value">{_esc(value)}</div>'
        f"</div>"
    )


def _render_metrics_section(
    report: dict, agent: dict, series: list[tuple[str, float]]
) -> str:
    if agent.get("final_nav_usd") is not None:
        final_nav = _format_nav(agent.get("final_nav_usd"))
    elif report.get("nav_usd") is not None:
        final_nav = _format_nav(report.get("nav_usd"))
    elif series:
        final_nav = _format_nav(series[-1][1])
    else:
        final_nav = PLACEHOLDER

    starting_nav = _format_nav(report.get("starting_nav_usd"))

    reported_drawdown = _format_drawdown_fraction(
        agent.get("total_drawdown_pct")
        if agent.get("total_drawdown_pct") is not None
        else report.get("total_drawdown_pct")
    )
    observed_drawdown = (
        _format_drawdown_fraction(max_drawdown([nav for _, nav in series]))
        if series
        else PLACEHOLDER
    )

    regime = _identity_value(report.get("regime"), agent.get("regime"))
    kill_switch_on = bool(report.get("kill_switch", False))
    kill_label = "TRIGGERED" if kill_switch_on else "inactive"
    kill_accent = "danger" if kill_switch_on else "ok"

    cards = [
        _render_metric_card("Starting NAV", starting_nav),
        _render_metric_card("Final NAV", final_nav, "primary"),
        _render_metric_card("Reported drawdown", reported_drawdown),
        _render_metric_card("Observed max drawdown", observed_drawdown),
        _render_metric_card("Regime", regime),
        _render_metric_card("Kill switch", kill_label, kill_accent),
    ]
    return (
        '<section class="card">'
        "<h2>Key Metrics</h2>"
        f'<div class="metrics">{"".join(cards)}</div>'
        "</section>"
    )


def _render_identity_section(report: dict, agent: dict) -> str:
    agent_id = _identity_value(
        agent.get("agent_id"), report.get("agent_id"), default=""
    )
    wallet = _identity_value(
        agent.get("wallet_address"),
        agent.get("wallet"),
        report.get("wallet_address"),
        report.get("wallet"),
    )
    policy_hash = _identity_value(
        agent.get("policy_hash"), report.get("policy_hash")
    )
    report_hash = _identity_value(
        agent.get("report_hash"), report.get("report_hash")
    )
    run_id = _identity_value(agent.get("run_id"), report.get("run_id"))

    rows = [
        ("Wallet", wallet),
        ("Policy hash", policy_hash),
        ("Report hash", report_hash),
        ("Run ID", run_id),
    ]
    if agent_id:
        rows.insert(0, ("Agent ID", agent_id))

    row_html = "".join(
        f"<tr><th>{_esc(label)}</th>"
        f'<td><code>{_esc(value)}</code></td></tr>'
        for label, value in rows
    )
    return (
        '<section class="card">'
        "<h2>Agent Identity &amp; Proof</h2>"
        '<table class="kv">'
        f"<tbody>{row_html}</tbody>"
        "</table>"
        "</section>"
    )


def _render_positions_section(report: dict) -> str:
    positions = report.get("positions") or []
    if not isinstance(positions, list) or not positions:
        return (
            '<section class="card">'
            "<h2>Positions</h2>"
            '<p class="empty">No open positions.</p>'
            "</section>"
        )

    ranked = sorted(positions, key=_position_weight, reverse=True)

    body_rows = []
    for position in ranked:
        if not isinstance(position, dict):
            continue
        symbol = _identity_value(position.get("symbol"), default="?")
        value = _format_nav(position.get("value_usd"))
        weight = _format_pct_value(position.get("weight_pct"))
        body_rows.append(
            f"<tr><td>{_esc(symbol)}</td>"
            f'<td class="num">{_esc(value)}</td>'
            f'<td class="num">{_esc(weight)}</td></tr>'
        )

    return (
        '<section class="card">'
        "<h2>Positions</h2>"
        '<table class="data">'
        "<thead><tr><th>Symbol</th>"
        '<th class="num">Value (USD)</th>'
        '<th class="num">Weight</th></tr></thead>'
        f"<tbody>{''.join(body_rows)}</tbody>"
        "</table>"
        "</section>"
    )


def _render_event_counts_section(counts: dict, total_events: int) -> str:
    if not counts:
        return (
            '<section class="card">'
            "<h2>Event-Type Counts</h2>"
            '<p class="empty">No events recorded.</p>'
            "</section>"
        )

    body_rows = "".join(
        f"<tr><td>{_esc(event_type)}</td>"
        f'<td class="num">{_esc(_format_int(count))}</td></tr>'
        for event_type, count in counts.items()
    )
    return (
        '<section class="card">'
        f"<h2>Event-Type Counts <span class=\"badge\">"
        f"{_esc(_format_int(total_events))} total</span></h2>"
        '<table class="data">'
        "<thead><tr><th>Event Type</th>"
        '<th class="num">Count</th></tr></thead>'
        f"<tbody>{body_rows}</tbody>"
        "</table>"
        "</section>"
    )


def _render_attribution_section(attribution: list[dict], trades: int) -> str:
    if not attribution:
        return (
            '<section class="card">'
            "<h2>Trade Attribution</h2>"
            '<p class="empty">No confirmed swaps recorded.</p>'
            "</section>"
        )

    body_rows = "".join(
        f"<tr><td>{_esc(_identity_value(row.get('symbol'), default='?'))}</td>"
        f'<td class="num">{_esc(_format_int(row.get("count")))}</td>'
        f'<td class="num">{_esc(_format_nav(row.get("total_amount_usd")))}</td></tr>'
        for row in attribution
    )
    return (
        '<section class="card">'
        f"<h2>Trade Attribution <span class=\"badge\">"
        f"{_esc(_format_int(trades))} confirmed</span></h2>"
        '<p class="hint">Confirmed swaps grouped by destination symbol.</p>'
        '<table class="data">'
        "<thead><tr><th>Destination</th>"
        '<th class="num">Confirmed Swaps</th>'
        '<th class="num">Total Amount (USD)</th></tr></thead>'
        f"<tbody>{body_rows}</tbody>"
        "</table>"
        "</section>"
    )


def _render_experiments_section(rows: list[dict]) -> str:
    if not rows:
        return (
            '<section class="card">'
            "<h2>Experiment Comparison</h2>"
            '<p class="empty">No experiments recorded.</p>'
            "</section>"
        )

    body_rows = []
    for row in rows:
        body_rows.append(
            "<tr>"
            f"<td>{_esc(_identity_value(row.get('tag'), default='?'))}</td>"
            f"<td>{_esc(_identity_value(row.get('preset'), default=''))}</td>"
            f'<td class="num">{_esc(_format_pct_value(row.get("total_return_pct")))}</td>'
            f'<td class="num">{_esc(_format_pct_value(row.get("excess_return_pct")))}</td>'
            f'<td class="num">{_esc(_format_pct_value(row.get("max_drawdown_pct")))}</td>'
            f'<td class="num">{_esc(_format_number(row.get("calmar_ratio")))}</td>'
            f'<td class="num">{_esc(_format_int(row.get("trade_count")))}</td>'
            f'<td class="num">{_esc(_format_nav(row.get("final_nav_usd")))}</td>'
            "</tr>"
        )
    return (
        '<section class="card">'
        "<h2>Experiment Comparison</h2>"
        '<table class="data">'
        "<thead><tr><th>Tag</th><th>Preset</th>"
        '<th class="num">Return</th>'
        '<th class="num">Excess</th>'
        '<th class="num">Max DD</th>'
        '<th class="num">Calmar</th>'
        '<th class="num">Trades</th>'
        '<th class="num">Final NAV</th></tr></thead>'
        f"<tbody>{''.join(body_rows)}</tbody>"
        "</table>"
        "</section>"
    )


def _render_regime_note(timeline: list[dict]) -> str:
    if not timeline:
        return ""
    first = timeline[0]
    last = timeline[-1]
    return (
        '<p class="hint">Regime classifications: '
        f"{_esc(_format_int(len(timeline)))} "
        f"(first <code>{_esc(_identity_value(first.get('regime'), default=UNKNOWN))}</code>"
        f" &rarr; last <code>{_esc(_identity_value(last.get('regime'), default=UNKNOWN))}</code>)."
        "</p>"
    )


_STYLE = """
:root {
  --bg: #0f1420;
  --panel: #19202e;
  --panel-alt: #212a3b;
  --text: #e6ecf5;
  --muted: #93a1b8;
  --accent: #5b9dff;
  --ok: #46c98b;
  --danger: #ff6b6b;
  --border: #2c3853;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  background: var(--bg);
  color: var(--text);
  line-height: 1.5;
}
.wrap { max-width: 1040px; margin: 0 auto; padding: 32px 20px 64px; }
header.report-header {
  border-bottom: 2px solid var(--border);
  padding-bottom: 20px;
  margin-bottom: 28px;
}
header.report-header h1 { margin: 0 0 6px; font-size: 28px; letter-spacing: -0.5px; }
header.report-header .sub { color: var(--muted); font-size: 14px; }
header.report-header .sub code { color: var(--accent); }
.pill {
  display: inline-block;
  padding: 2px 10px;
  border-radius: 999px;
  background: var(--panel-alt);
  border: 1px solid var(--border);
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--muted);
  margin-left: 8px;
}
.card {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 20px 22px;
  margin-bottom: 22px;
}
.card h2 {
  margin: 0 0 14px;
  font-size: 17px;
  display: flex;
  align-items: center;
  gap: 10px;
}
.badge {
  font-size: 12px;
  font-weight: 500;
  color: var(--muted);
  background: var(--panel-alt);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 1px 8px;
}
.metrics {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 12px;
}
.metric {
  background: var(--panel-alt);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 16px;
}
.metric__label {
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--muted);
  margin-bottom: 6px;
}
.metric__value { font-size: 20px; font-weight: 600; }
.metric--primary .metric__value { color: var(--accent); }
.metric--ok .metric__value { color: var(--ok); }
.metric--danger .metric__value { color: var(--danger); }
table { width: 100%; border-collapse: collapse; font-size: 14px; }
table.data thead th {
  text-align: left;
  color: var(--muted);
  font-weight: 600;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 8px 10px;
  border-bottom: 1px solid var(--border);
}
table.data tbody td {
  padding: 8px 10px;
  border-bottom: 1px solid var(--border);
}
table.data tbody tr:last-child td { border-bottom: none; }
table.data tbody tr:nth-child(even) { background: rgba(255,255,255,0.02); }
.num { text-align: right; font-variant-numeric: tabular-nums; }
table.kv th {
  text-align: left;
  color: var(--muted);
  font-weight: 500;
  padding: 6px 16px 6px 0;
  white-space: nowrap;
  vertical-align: top;
}
table.kv td { padding: 6px 0; }
code {
  font-family: "SFMono-Regular", Menlo, Consolas, monospace;
  font-size: 13px;
  word-break: break-all;
  color: var(--text);
}
.empty, .hint { color: var(--muted); font-size: 13px; margin: 4px 0 0; }
.hint { margin-bottom: 12px; }
footer.report-footer {
  margin-top: 28px;
  color: var(--muted);
  font-size: 12px;
  text-align: center;
}
""".strip()


def _render_header(report: dict, agent: dict) -> str:
    run_id = _identity_value(report.get("run_id"), agent.get("run_id"))
    mode = _identity_value(report.get("mode"), agent.get("mode"))
    return (
        '<header class="report-header">'
        "<h1>Guardrail Alpha &mdash; Submission Report"
        f'<span class="pill">{_esc(mode)}</span></h1>'
        f'<div class="sub">Run ID: <code>{_esc(run_id)}</code></div>'
        "</header>"
    )


def build_submission_html(
    db_path: str = "data/guardrail_alpha.db",
    report_path: str = "data/run_report.json",
    experiments_dir: str = "data/experiments",
) -> str:
    """Build a self-contained HTML submission report for judges.

    Reads the event log, the agent run report, and the per-experiment JSON
    files, reusing the existing ``guardrail_lab`` helpers, and renders a single
    HTML document with inline styles and no external assets. Every section
    degrades gracefully when its data is missing, and all dynamic text is
    escaped via :func:`html.escape`.
    """
    events = load_events(db_path)
    report = load_run_report(report_path) or {}
    agent = _agent_report_payload(events)

    counts = event_counts(events)
    trades = trade_count(events)
    series = nav_series(events)
    attribution = trade_attribution(events)
    timeline = regime_timeline(events)
    experiment_rows = compare_table(load_experiments(experiments_dir))

    generated_at = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC")

    body_parts = [
        _render_header(report, agent),
        _render_metrics_section(report, agent, series),
        _render_identity_section(report, agent),
        _render_positions_section(report),
        _render_attribution_section(attribution, trades),
        _render_event_counts_section(counts, len(events)),
        _render_experiments_section(experiment_rows),
    ]

    regime_note = _render_regime_note(timeline)
    if regime_note:
        body_parts.append(f'<section class="card"><h2>Regime</h2>{regime_note}</section>')

    body_parts.append(
        '<footer class="report-footer">'
        f"Generated {_esc(generated_at)} &middot; "
        "Guardrail Alpha lab &middot; stdlib-only, self-contained."
        "</footer>"
    )

    return (
        "<!doctype html>\n"
        '<html lang="en">\n'
        "<head>\n"
        '<meta charset="utf-8">\n'
        '<meta name="viewport" content="width=device-width, initial-scale=1">\n'
        "<title>Guardrail Alpha &mdash; Submission Report</title>\n"
        f"<style>\n{_STYLE}\n</style>\n"
        "</head>\n"
        "<body>\n"
        f'<div class="wrap">\n{"".join(body_parts)}\n</div>\n'
        "</body>\n"
        "</html>\n"
    )
