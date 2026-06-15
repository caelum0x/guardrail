//! Screen rendering for the cockpit. Produces a single string per refresh that
//! clears the terminal and draws the header, run summary, positions, and event
//! totals.

use std::time::{SystemTime, UNIX_EPOCH};

use event_store::StoredEvent;

use crate::alerts::AlertsPanel;
use crate::positions::PositionsSummary;
use crate::regime::RegimePanel;
use crate::report::RunReport;
use crate::risk::RiskPanel;
use crate::totals::EventTotals;

/// ANSI clear-screen + cursor-home sequence.
const CLEAR: &str = "\x1b[2J\x1b[H";
const WIDTH: usize = 60;

/// A renderable snapshot of the cockpit for one refresh cycle.
pub struct Screen<'a> {
    report: &'a RunReport,
    totals: &'a EventTotals,
    regime: RegimePanel,
    positions_summary: PositionsSummary,
    risk: RiskPanel,
    alerts: AlertsPanel,
    cycle: u32,
    total_cycles: u32,
}

impl<'a> Screen<'a> {
    pub fn new(
        report: &'a RunReport,
        totals: &'a EventTotals,
        events: &Option<Vec<StoredEvent>>,
        policy_path: &str,
        cycle: u32,
        total_cycles: u32,
    ) -> Self {
        let event_slice: &[StoredEvent] = events.as_deref().unwrap_or(&[]);

        // The regime is meaningful when we have either an event log or a report
        // to source it from; otherwise nothing is known.
        let regime = if events.is_some() || report.available {
            RegimePanel::from_sources(event_slice, &report.regime)
        } else {
            RegimePanel::unavailable()
        };

        let positions_summary = if report.available {
            PositionsSummary::from_positions(&report.positions)
        } else {
            PositionsSummary::empty()
        };

        let risk = match events {
            Some(events) => RiskPanel::from_events(events),
            None => RiskPanel::unavailable(),
        };

        let alerts = AlertsPanel::build(
            report.available,
            &report.kill_switch,
            &report.total_drawdown_pct,
            event_slice,
            policy_path,
        );

        Self {
            report,
            totals,
            regime,
            positions_summary,
            risk,
            alerts,
            cycle,
            total_cycles,
        }
    }

    /// Renders the full screen as a string ready to print.
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(2048);
        out.push_str(CLEAR);
        self.render_header(&mut out);
        self.render_summary(&mut out);
        self.render_regime(&mut out);
        self.render_positions(&mut out);
        self.render_risk(&mut out);
        self.render_alerts(&mut out);
        self.render_totals(&mut out);
        self.render_footer(&mut out);
        out
    }

    fn render_header(&self, out: &mut String) {
        push_rule(out);
        push_line(out, "Guardrail Alpha — cockpit");
        push_line(out, &format!("ts: {}", unix_timestamp()));
        push_line(
            out,
            &format!("refresh {} / {}", self.cycle, self.total_cycles),
        );
        push_rule(out);
    }

    fn render_summary(&self, out: &mut String) {
        if !self.report.available {
            push_line(out, "RUN REPORT: unavailable (placeholder)");
            push_rule(out);
            return;
        }
        push_line(out, "RUN REPORT");
        push_kv(out, "run_id", &self.report.run_id);
        push_kv(out, "mode", &self.report.mode);
        push_kv(out, "regime", &self.report.regime);
        push_kv(out, "nav_usd", &self.report.nav_usd);
        push_kv(out, "drawdown_pct", &self.report.total_drawdown_pct);
        push_kv(out, "kill_switch", &self.report.kill_switch);
        push_rule(out);
    }

    fn render_regime(&self, out: &mut String) {
        if !self.regime.available {
            push_line(out, "REGIME: unavailable (placeholder)");
            push_rule(out);
            return;
        }
        push_line(out, "REGIME");
        push_kv(out, "classification", &self.regime.regime);
        push_kv(out, "exposure multiplier", &self.regime.exposure);
        push_kv(out, "source", &self.regime.source);
        push_rule(out);
    }

    fn render_positions(&self, out: &mut String) {
        push_line(out, "POSITIONS");
        if self.report.positions.is_empty() {
            push_line(out, "  (none)");
            push_rule(out);
            return;
        }
        if self.positions_summary.available {
            push_kv(out, "holdings", &self.positions_summary.count.to_string());
            push_kv(
                out,
                "total value_usd",
                &self.positions_summary.total_value_usd,
            );
            push_kv(
                out,
                "total weight%",
                &self.positions_summary.total_weight_pct,
            );
            push_kv(
                out,
                "top holding",
                &format!(
                    "{} ({}%)",
                    self.positions_summary.top_symbol, self.positions_summary.top_weight_pct
                ),
            );
        }
        push_line(
            out,
            &format!("  {:<10} {:>14} {:>10}", "SYMBOL", "VALUE_USD", "WEIGHT%"),
        );
        for position in &self.report.positions {
            push_line(
                out,
                &format!(
                    "  {:<10} {:>14} {:>10}",
                    truncate(&position.symbol, 10),
                    truncate(&position.value_usd, 14),
                    truncate(&position.weight_pct, 10),
                ),
            );
        }
        push_rule(out);
    }

    fn render_risk(&self, out: &mut String) {
        if !self.risk.available {
            push_line(out, "RISK: unavailable (placeholder)");
            push_rule(out);
            return;
        }
        push_line(out, &format!("RISK (last {} decisions)", self.risk.total()));
        push_kv(out, "approved", &self.risk.approved.to_string());
        push_kv(out, "clipped", &self.risk.clipped.to_string());
        push_kv(out, "rejected", &self.risk.rejected.to_string());
        push_line(out, "  recent reasons:");
        if self.risk.recent_reasons.is_empty() {
            push_line(out, "    (none)");
        } else {
            for reason in &self.risk.recent_reasons {
                push_line(out, &format!("    - {}", truncate(reason, WIDTH - 6)));
            }
        }
        push_rule(out);
    }

    fn render_alerts(&self, out: &mut String) {
        if !self.alerts.available {
            push_line(out, "ALERTS / READINESS: unavailable (placeholder)");
            push_rule(out);
            return;
        }
        push_line(out, "ALERTS / READINESS");
        for row in &self.alerts.rows {
            push_kv(out, &row.label, &row.status);
        }
        push_rule(out);
    }

    fn render_totals(&self, out: &mut String) {
        if !self.totals.available {
            push_line(out, "EVENT TOTALS: unavailable (placeholder)");
            push_rule(out);
            return;
        }
        push_line(
            out,
            &format!("EVENT TOTALS (last {} events)", self.totals.total),
        );
        push_kv(
            out,
            "trades (tx_confirmed)",
            &self.totals.trades().to_string(),
        );
        push_kv(
            out,
            "rejections (risk_rejected)",
            &self.totals.rejections().to_string(),
        );
        push_line(out, "  by type:");
        if self.totals.by_type.is_empty() {
            push_line(out, "    (no events)");
        } else {
            for (name, count) in &self.totals.by_type {
                push_line(out, &format!("    {:<28} {:>5}", truncate(name, 28), count));
            }
        }
        push_rule(out);
    }

    fn render_footer(&self, out: &mut String) {
        push_line(out, "polling cockpit — exits after final refresh");
        push_rule(out);
    }
}

/// Current unix time in seconds (best-effort; 0 if the clock is before the
/// epoch, which should never happen).
fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn push_rule(out: &mut String) {
    out.push_str(&"=".repeat(WIDTH));
    out.push('\n');
}

fn push_line(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}

fn push_kv(out: &mut String, key: &str, value: &str) {
    out.push_str(&format!("  {:<26} {}\n", key, value));
}

/// Truncates a string to `max` chars, appending an ellipsis marker when cut.
fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut result: String = text.chars().take(max.saturating_sub(1)).collect();
    result.push('…');
    result
}
