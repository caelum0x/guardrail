"""Plain-text summary renderer for terminal / log output."""

from __future__ import annotations

from datetime import datetime, timezone
from typing import Optional

from .data import EventLog, RunReport
from .format import fmt_int, fmt_money, fmt_pct, fmt_ratio, fmt_str
from .metrics import ReportMetrics

_RULE = "=" * 60
_SUB = "-" * 60


def render_text(
    event_log: EventLog,
    metrics: ReportMetrics,
    run_report: Optional[RunReport] = None,
    *,
    generated_at: Optional[datetime] = None,
) -> str:
    """Render a concise text summary of the run."""
    when = (generated_at or datetime.now(timezone.utc)).strftime("%Y-%m-%d %H:%M:%S UTC")
    run_id = (run_report.run_id if run_report else None) or (
        event_log.run_ids[0] if event_log.run_ids else "unknown"
    )

    lines: list[str] = []
    lines.append(_RULE)
    lines.append("GUARDRAIL RUN REPORT")
    lines.append(_RULE)
    lines.append(f"Run id        : {run_id}")
    if run_report is not None:
        lines.append(f"Mode          : {fmt_str(run_report.mode)}")
        lines.append(f"Regime        : {fmt_str(run_report.regime)}")
        if run_report.kill_switch is not None:
            lines.append(f"Kill switch   : {'ON' if run_report.kill_switch else 'off'}")
    lines.append(f"DB            : {event_log.db_path}")
    if run_report is not None:
        lines.append(f"Run report    : {run_report.path}")
    lines.append("")

    lines.append("SUMMARY RISK METRICS")
    lines.append(_SUB)
    lines.append(f"  Starting NAV     : {fmt_money(metrics.first_nav)}")
    lines.append(f"  Ending NAV       : {fmt_money(metrics.last_nav)}")
    lines.append(f"  Peak NAV         : {fmt_money(metrics.peak_nav)}")
    lines.append(f"  Trough NAV       : {fmt_money(metrics.trough_nav)}")
    lines.append(f"  Total return     : {fmt_pct(metrics.total_return_pct)}")
    lines.append(f"  Max drawdown     : {fmt_pct(metrics.max_drawdown_pct)}")
    lines.append(f"  Volatility       : {fmt_pct(metrics.volatility_pct)}")
    lines.append(f"  Simple sharpe    : {fmt_ratio(metrics.sharpe)}")
    lines.append(f"  NAV points       : {fmt_int(metrics.nav_points)}")
    lines.append(f"  Confirmed trades : {fmt_int(metrics.confirmed_trades)}")
    lines.append(f"  Total events     : {fmt_int(metrics.total_events)}")
    lines.append("")

    lines.append("EVENT COUNTS")
    lines.append(_SUB)
    counts = event_log.event_counts
    if counts:
        width = max(len(k) for k in counts)
        for et in sorted(counts, key=lambda k: (-counts[k], k)):
            lines.append(f"  {et:<{width}}  {counts[et]}")
    else:
        lines.append("  (no events)")
    lines.append("")

    if run_report is not None and run_report.positions:
        lines.append("POSITIONS")
        lines.append(_SUB)
        for p in run_report.positions:
            lines.append(
                f"  {p.symbol:<8} {fmt_money(p.value_usd):>14}  "
                f"{fmt_pct(p.weight_pct, places=2):>8}"
            )
        lines.append("")

    lines.append(f"Generated {when}")
    lines.append(_RULE)
    return "\n".join(lines) + "\n"
