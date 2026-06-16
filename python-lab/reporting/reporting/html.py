"""Self-contained HTML report renderer.

Produces a single HTML document with inline CSS and an inline SVG NAV chart —
no external assets, no JavaScript, no dependencies. Everything user-derived is
HTML-escaped before it reaches the document.
"""

from __future__ import annotations

import html
from datetime import datetime, timezone
from decimal import Decimal
from typing import Optional, Sequence

from .data import EventLog, NavPoint, RunReport
from .format import (
    fmt_int,
    fmt_money,
    fmt_pct,
    fmt_ratio,
    fmt_str,
    sign_class,
)
from .metrics import ReportMetrics

_CSS = """
:root {
  --bg: #0f1419; --panel: #171d26; --panel2: #1f2733; --line: #2a3441;
  --text: #e6edf3; --muted: #8b98a9; --accent: #4ea1ff;
  --pos: #3fb950; --neg: #f85149; --flat: #8b98a9;
}
* { box-sizing: border-box; }
body {
  margin: 0; background: var(--bg); color: var(--text);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  line-height: 1.5; -webkit-font-smoothing: antialiased;
}
.wrap { max-width: 960px; margin: 0 auto; padding: 32px 24px 64px; }
header h1 { margin: 0 0 4px; font-size: 24px; letter-spacing: -0.01em; }
header .sub { color: var(--muted); font-size: 13px; }
.badges { margin: 16px 0 8px; }
.badge {
  display: inline-block; padding: 3px 10px; border-radius: 999px;
  font-size: 12px; font-weight: 600; margin: 0 6px 6px 0;
  background: var(--panel2); color: var(--text); border: 1px solid var(--line);
}
.badge.kill-on { background: #3d1418; color: var(--neg); border-color: #5e1f24; }
.badge.kill-off { background: #11301a; color: var(--pos); border-color: #1c4a2a; }
section { margin-top: 32px; }
section h2 {
  font-size: 13px; text-transform: uppercase; letter-spacing: 0.08em;
  color: var(--muted); margin: 0 0 12px; font-weight: 600;
}
.cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; }
.card {
  background: var(--panel); border: 1px solid var(--line);
  border-radius: 10px; padding: 14px 16px;
}
.card .label { font-size: 11px; color: var(--muted); text-transform: uppercase; letter-spacing: 0.06em; }
.card .value { font-size: 22px; font-weight: 700; margin-top: 4px; font-variant-numeric: tabular-nums; }
.pos { color: var(--pos); } .neg { color: var(--neg); } .flat { color: var(--text); }
table { width: 100%; border-collapse: collapse; font-size: 13px; }
th, td { text-align: left; padding: 8px 10px; border-bottom: 1px solid var(--line); }
th { color: var(--muted); font-weight: 600; font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; }
td.num, th.num { text-align: right; font-variant-numeric: tabular-nums; }
.mono { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 12px; }
.panel { background: var(--panel); border: 1px solid var(--line); border-radius: 10px; padding: 16px; overflow-x: auto; }
.chart { width: 100%; height: auto; display: block; }
.empty { color: var(--muted); font-style: italic; padding: 8px 0; }
footer { margin-top: 48px; color: var(--muted); font-size: 12px; border-top: 1px solid var(--line); padding-top: 16px; }
a { color: var(--accent); }
"""


def _esc(value: object) -> str:
    return html.escape("" if value is None else str(value), quote=True)


def _nav_chart_svg(points: Sequence[NavPoint], width: int = 880, height: int = 240) -> str:
    """Render the NAV series as an inline SVG line chart.

    Pure geometry: maps NAV values to a padded plot area and draws a polyline.
    """
    if len(points) < 2:
        return '<p class="empty">Not enough NAV observations to plot a chart.</p>'

    navs = [p.nav for p in points]
    lo = min(navs)
    hi = max(navs)
    pad_l, pad_r, pad_t, pad_b = 56, 16, 16, 28
    plot_w = width - pad_l - pad_r
    plot_h = height - pad_t - pad_b

    span = hi - lo
    if span == 0:
        span = Decimal(1)  # flat line: avoid divide-by-zero, centre it

    n = len(navs)

    def x(i: int) -> float:
        return pad_l + (plot_w * i / (n - 1))

    def y(v: Decimal) -> float:
        frac = float((v - lo) / span)
        return pad_t + plot_h * (1 - frac)

    pts = " ".join(f"{x(i):.2f},{y(v):.2f}" for i, v in enumerate(navs))

    # Area path under the line for a subtle fill.
    area = (
        f"M {x(0):.2f},{pad_t + plot_h:.2f} "
        + " ".join(f"L {x(i):.2f},{y(v):.2f}" for i, v in enumerate(navs))
        + f" L {x(n - 1):.2f},{pad_t + plot_h:.2f} Z"
    )

    last = navs[-1]
    first = navs[0]
    stroke = "#3fb950" if last >= first else "#f85149"
    fill = "rgba(63,185,80,0.12)" if last >= first else "rgba(248,81,73,0.12)"

    # Y-axis gridlines / labels at min, mid, max.
    mid = lo + span / 2
    labels = []
    for val in (hi, mid, lo):
        yy = y(val)
        labels.append(
            f'<line x1="{pad_l}" y1="{yy:.2f}" x2="{width - pad_r}" y2="{yy:.2f}" '
            f'stroke="#2a3441" stroke-width="1"/>'
        )
        labels.append(
            f'<text x="{pad_l - 8}" y="{yy + 4:.2f}" text-anchor="end" '
            f'fill="#8b98a9" font-size="10">{_esc(f"{val:,.2f}")}</text>'
        )

    return (
        f'<svg class="chart" viewBox="0 0 {width} {height}" '
        f'preserveAspectRatio="none" role="img" '
        f'aria-label="NAV over time">'
        f"{''.join(labels)}"
        f'<path d="{area}" fill="{fill}" stroke="none"/>'
        f'<polyline points="{pts}" fill="none" stroke="{stroke}" '
        f'stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>'
        f"</svg>"
    )


def _metric_card(label: str, value: str, css: str = "flat") -> str:
    return (
        f'<div class="card"><div class="label">{_esc(label)}</div>'
        f'<div class="value {css}">{_esc(value)}</div></div>'
    )


def _metrics_section(m: ReportMetrics) -> str:
    cards = [
        _metric_card("Starting NAV", fmt_money(m.first_nav)),
        _metric_card("Ending NAV", fmt_money(m.last_nav)),
        _metric_card(
            "Total Return",
            fmt_pct(m.total_return_pct),
            sign_class(m.total_return),
        ),
        _metric_card(
            "Max Drawdown",
            fmt_pct(m.max_drawdown_pct),
            "neg" if (m.max_drawdown and m.max_drawdown > 0) else "flat",
        ),
        _metric_card("Simple Sharpe", fmt_ratio(m.sharpe), sign_class(m.sharpe)),
        _metric_card("Volatility", fmt_pct(m.volatility_pct)),
        _metric_card("NAV Points", fmt_int(m.nav_points)),
        _metric_card("Confirmed Trades", fmt_int(m.confirmed_trades)),
    ]
    return (
        "<section><h2>Summary Risk Metrics</h2>"
        f'<div class="cards">{"".join(cards)}</div></section>'
    )


def _event_counts_section(event_log: EventLog) -> str:
    counts = event_log.event_counts
    if not counts:
        return (
            "<section><h2>Event Counts</h2>"
            '<p class="empty">No events found.</p></section>'
        )
    rows = []
    for et in sorted(counts, key=lambda k: (-counts[k], k)):
        rows.append(
            f"<tr><td class='mono'>{_esc(et)}</td>"
            f"<td class='num'>{fmt_int(counts[et])}</td></tr>"
        )
    return (
        "<section><h2>Event Counts</h2><div class='panel'><table>"
        "<thead><tr><th>Event Type</th><th class='num'>Count</th></tr></thead>"
        f"<tbody>{''.join(rows)}</tbody>"
        f"<tfoot><tr><th>Total</th><th class='num'>{fmt_int(event_log.total_events)}</th></tr></tfoot>"
        "</table></div></section>"
    )


def _nav_section(event_log: EventLog) -> str:
    chart = _nav_chart_svg(event_log.nav_series)
    return (
        "<section><h2>NAV Over Time</h2>"
        f"<div class='panel'>{chart}</div></section>"
    )


def _trades_section(event_log: EventLog) -> str:
    trades = event_log.confirmed_trades
    if not trades:
        return (
            "<section><h2>Confirmed Trades</h2>"
            '<p class="empty">No confirmed transactions.</p></section>'
        )
    rows = []
    for t in trades:
        ident = t.tx_hash or t.competition_tx or "—"
        rows.append(
            "<tr>"
            f"<td class='mono'>{_esc(t.timestamp_raw)}</td>"
            f"<td class='mono'>{_esc(ident)}</td>"
            f"<td>{_esc(fmt_str(t.status))}</td>"
            f"<td class='num'>{_esc(fmt_int(t.block))}</td>"
            "</tr>"
        )
    return (
        "<section><h2>Confirmed Trades</h2><div class='panel'><table>"
        "<thead><tr><th>Timestamp</th><th>Tx / Competition Id</th>"
        "<th>Status</th><th class='num'>Block</th></tr></thead>"
        f"<tbody>{''.join(rows)}</tbody></table></div></section>"
    )


def _positions_section(report: Optional[RunReport]) -> str:
    if report is None or not report.positions:
        return ""
    rows = []
    for p in report.positions:
        rows.append(
            "<tr>"
            f"<td class='mono'>{_esc(p.symbol)}</td>"
            f"<td class='num'>{_esc(fmt_money(p.value_usd))}</td>"
            f"<td class='num'>{_esc(fmt_pct(p.weight_pct, places=2))}</td>"
            "</tr>"
        )
    return (
        "<section><h2>Positions (Run Report)</h2><div class='panel'><table>"
        "<thead><tr><th>Symbol</th><th class='num'>Value</th>"
        "<th class='num'>Weight</th></tr></thead>"
        f"<tbody>{''.join(rows)}</tbody></table></div></section>"
    )


def _header(report: Optional[RunReport], event_log: EventLog) -> str:
    run_id = None
    mode = regime = None
    kill: Optional[bool] = None
    if report is not None:
        run_id = report.run_id
        mode = report.mode
        regime = report.regime
        kill = report.kill_switch
    if not run_id and event_log.run_ids:
        run_id = event_log.run_ids[0]

    badges = []
    if mode:
        badges.append(f'<span class="badge">mode: {_esc(mode)}</span>')
    if regime:
        badges.append(f'<span class="badge">regime: {_esc(regime)}</span>')
    if kill is True:
        badges.append('<span class="badge kill-on">kill switch: ON</span>')
    elif kill is False:
        badges.append('<span class="badge kill-off">kill switch: off</span>')

    sub = f"Run {_esc(run_id)}" if run_id else "Run (id unknown)"
    return (
        "<header><h1>Guardrail Run Report</h1>"
        f'<div class="sub">{sub}</div>'
        f'<div class="badges">{"".join(badges)}</div></header>'
    )


def render_html(
    event_log: EventLog,
    metrics: ReportMetrics,
    run_report: Optional[RunReport] = None,
    *,
    title: str = "Guardrail Run Report",
    generated_at: Optional[datetime] = None,
) -> str:
    """Render the full self-contained HTML document as a string."""
    when = (generated_at or datetime.now(timezone.utc)).strftime("%Y-%m-%d %H:%M:%S UTC")

    body = "".join(
        [
            _header(run_report, event_log),
            _metrics_section(metrics),
            _nav_section(event_log),
            _positions_section(run_report),
            _event_counts_section(event_log),
            _trades_section(event_log),
            "<footer>Generated "
            f"{_esc(when)} from <span class='mono'>{_esc(event_log.db_path)}</span>"
            + (
                f" and <span class='mono'>{_esc(run_report.path)}</span>"
                if run_report is not None
                else ""
            )
            + ". Self-contained report — no external assets.</footer>",
        ]
    )

    return (
        "<!DOCTYPE html>\n"
        '<html lang="en"><head><meta charset="utf-8"/>'
        '<meta name="viewport" content="width=device-width, initial-scale=1"/>'
        f"<title>{_esc(title)}</title>"
        f"<style>{_CSS}</style></head>"
        f'<body><div class="wrap">{body}</div></body></html>\n'
    )
