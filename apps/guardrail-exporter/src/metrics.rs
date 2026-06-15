//! Prometheus text exposition rendering.

use crate::config::Config;
use crate::counts::{event_counts, Counts};
use crate::report::{load_report, now_ms, num};

/// Append one gauge metric (HELP + TYPE + sample) to the buffer.
pub fn gauge(out: &mut String, name: &str, help: &str, value: f64) {
    out.push_str(&format!("# HELP {name} {help}\n"));
    out.push_str(&format!("# TYPE {name} gauge\n"));
    out.push_str(&format!("{name} {value}\n"));
}

/// Append one counter metric (HELP + TYPE + sample) to the buffer.
pub fn counter(out: &mut String, name: &str, help: &str, value: f64) {
    out.push_str(&format!("# HELP {name} {help}\n"));
    out.push_str(&format!("# TYPE {name} counter\n"));
    out.push_str(&format!("{name} {value}\n"));
}

/// Parse an ISO-8601 timestamp to epoch millis (for age computation).
fn iso_to_ms(ts: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

/// Build the full Prometheus exposition body.
pub fn render(cfg: &Config) -> String {
    let started = now_ms();
    let mut out = String::new();

    let counts = event_counts(&cfg.db_path);
    render_event_counts(&mut out, &counts);

    if let Some(regime) = &counts.latest_regime {
        out.push_str("# HELP guardrail_regime Current market regime (value is always 1; see label)\n");
        out.push_str("# TYPE guardrail_regime gauge\n");
        out.push_str(&format!("guardrail_regime{{regime=\"{regime}\"}} 1\n"));
    }

    if let Some(ts) = counts.last_event_ts.as_deref().and_then(iso_to_ms) {
        let age = ((started - ts).max(0)) as f64 / 1000.0;
        gauge(&mut out, "guardrail_last_event_age_seconds", "Seconds since the newest event", age);
    }

    if let Some(report) = load_report(&cfg.report_path) {
        render_report(&mut out, &report, started);
    }

    // Build info (constant gauge labeled with the crate version).
    out.push_str("# HELP guardrail_build_info Exporter build info (value always 1)\n");
    out.push_str("# TYPE guardrail_build_info gauge\n");
    out.push_str(&format!(
        "guardrail_build_info{{version=\"{}\"}} 1\n",
        env!("CARGO_PKG_VERSION")
    ));

    let scrape_seconds = ((now_ms() - started).max(0)) as f64 / 1000.0;
    gauge(&mut out, "guardrail_scrape_duration_seconds", "Time to build this exposition", scrape_seconds);

    out
}

/// Counters derived from the event log.
fn render_event_counts(out: &mut String, c: &Counts) {
    gauge(out, "guardrail_events_total", "Total recorded agent events", c.events as f64);
    counter(out, "guardrail_orders_proposed_total", "Orders proposed by the strategy", c.proposed as f64);
    counter(out, "guardrail_risk_approved_total", "Orders approved by the risk engine", c.approved as f64);
    counter(out, "guardrail_risk_rejections_total", "Orders rejected by the risk engine", c.rejections as f64);
    counter(out, "guardrail_risk_clips_total", "Orders clipped (resized) by the risk engine", c.clips as f64);
    counter(out, "guardrail_quotes_total", "TWAK quotes received", c.quotes as f64);
    counter(out, "guardrail_swaps_submitted_total", "TWAK swaps submitted", c.submitted as f64);
    counter(out, "guardrail_trades_total", "Confirmed on-chain swaps", c.trades as f64);
    counter(out, "guardrail_reconciliations_total", "Portfolio reconciliations", c.reconciled as f64);
    counter(out, "guardrail_daily_trade_satisfied_total", "Cycles satisfying the daily-trade requirement", c.daily_satisfied as f64);
    counter(out, "guardrail_throttle_activations_total", "Drawdown throttle activations", c.throttle_activations as f64);
    counter(out, "guardrail_kill_switch_triggered_total", "Kill-switch trigger events", c.kill_switches as f64);
}

/// Gauges derived from the latest run report.
fn render_report(out: &mut String, report: &serde_json::Value, now: i64) {
    if let Some(v) = num(report, "nav_usd") {
        gauge(out, "guardrail_nav_usd", "Net asset value in USD", v);
    }
    if let Some(v) = num(report, "total_drawdown_pct") {
        gauge(out, "guardrail_total_drawdown_pct", "Total drawdown percent", v);
    }

    let position_arr = report.get("positions").and_then(|p| p.as_array());
    let positions = position_arr.map(|a| a.len()).unwrap_or(0);
    gauge(out, "guardrail_positions", "Open non-reserve positions", positions as f64);

    if let Some(arr) = position_arr {
        out.push_str("# HELP guardrail_position_weight_pct Position weight as percent of NAV\n");
        out.push_str("# TYPE guardrail_position_weight_pct gauge\n");
        for p in arr {
            let symbol = p.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
            let weight = p
                .get("weight_pct")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            if !symbol.is_empty() {
                out.push_str(&format!(
                    "guardrail_position_weight_pct{{symbol=\"{symbol}\"}} {weight}\n"
                ));
            }
        }
    }

    let kill = report.get("kill_switch").and_then(|k| k.as_bool()).unwrap_or(false);
    gauge(out, "guardrail_kill_switch", "Kill switch engaged (1) or armed (0)", if kill { 1.0 } else { 0.0 });

    if let Some(updated_ms) = report.get("updated_ms").and_then(|v| v.as_i64()) {
        let age = ((now - updated_ms).max(0)) as f64 / 1000.0;
        gauge(out, "guardrail_report_age_seconds", "Seconds since the last run report", age);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_render_in_exposition_format() {
        let c = Counts {
            events: 5,
            trades: 2,
            ..Default::default()
        };
        let mut out = String::new();
        render_event_counts(&mut out, &c);
        assert!(out.contains("# TYPE guardrail_events_total gauge"));
        assert!(out.contains("guardrail_events_total 5"));
        assert!(out.contains("# TYPE guardrail_trades_total counter"));
        assert!(out.contains("guardrail_trades_total 2"));
    }

    #[test]
    fn iso_parses_to_millis() {
        assert_eq!(iso_to_ms("2026-01-01T00:00:00Z"), Some(1_767_225_600_000));
        assert_eq!(iso_to_ms("not-a-date"), None);
    }
}
