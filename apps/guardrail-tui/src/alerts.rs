//! Alerts / readiness panel model.
//!
//! Summarizes operational readiness: kill-switch state, current drawdown
//! against the throttle / kill thresholds from the risk policy, and daily-trade
//! status. Inputs come from the run report (kill-switch flag, drawdown) and the
//! event log (throttle activations, daily-trade satisfaction, kill-switch
//! triggers). Thresholds are read from the risk-policy JSON when available and
//! otherwise fall back to conservative defaults, so the panel works offline.

use serde_json::Value;

use event_store::{AgentEvent, StoredEvent};

/// Default throttle / kill thresholds (percent) used when the risk policy file
/// is missing or unparsable. Conservative values mirroring the paper policy.
const DEFAULT_THROTTLE_PCT: f64 = 22.0;
const DEFAULT_KILL_PCT: f64 = 24.0;

/// A single readiness indicator with a status label.
#[derive(Debug, Clone)]
pub struct Readiness {
    pub label: String,
    pub status: String,
}

/// Parsed view of operational readiness.
#[derive(Debug, Clone)]
pub struct AlertsPanel {
    pub available: bool,
    pub rows: Vec<Readiness>,
}

impl AlertsPanel {
    /// Placeholder used when neither the report nor the event log is available.
    pub fn unavailable() -> Self {
        Self {
            available: false,
            rows: Vec::new(),
        }
    }

    /// Builds the readiness panel.
    ///
    /// - `report_available` gates whether report-derived rows are meaningful.
    /// - `kill_switch` / `drawdown_pct` come from the run report (the drawdown
    ///   string may be a fraction such as `0.0644` or a percent such as `6.44`).
    /// - `events` (newest-first) supply throttle / kill / daily-trade signals.
    /// - `policy_path` is the risk-policy JSON used to resolve thresholds.
    pub fn build(
        report_available: bool,
        kill_switch: &str,
        drawdown_pct: &str,
        events: &[StoredEvent],
        policy_path: &str,
    ) -> Self {
        if !report_available && events.is_empty() {
            return Self::unavailable();
        }

        let (throttle_pct, kill_pct) = load_thresholds(policy_path);
        let rows = vec![
            kill_switch_row(kill_switch, events),
            drawdown_row(drawdown_pct, throttle_pct, kill_pct),
            daily_trade_row(events),
        ];

        Self {
            available: true,
            rows,
        }
    }
}

/// Builds the kill-switch readiness row, escalating to TRIGGERED if any
/// kill-switch event is present in the log.
fn kill_switch_row(kill_switch: &str, events: &[StoredEvent]) -> Readiness {
    let triggered_in_log = events
        .iter()
        .any(|e| matches!(e.event_type, AgentEvent::KillSwitchTriggered));

    let status = if triggered_in_log || matches!(kill_switch.trim(), "true" | "TRUE" | "1") {
        "TRIGGERED".to_string()
    } else if kill_switch.trim() == "—" {
        "unknown".to_string()
    } else {
        "armed (clear)".to_string()
    };

    Readiness {
        label: "kill-switch".to_string(),
        status,
    }
}

/// Builds the drawdown-vs-throttle readiness row.
fn drawdown_row(drawdown_pct: &str, throttle_pct: f64, kill_pct: f64) -> Readiness {
    let status = match normalize_pct(drawdown_pct) {
        Some(dd) => {
            let level = if dd >= kill_pct {
                "KILL"
            } else if dd >= throttle_pct {
                "THROTTLED"
            } else {
                "ok"
            };
            format!("{dd:.2}% / throttle {throttle_pct:.0}% — {level}")
        }
        None => "unknown".to_string(),
    };

    Readiness {
        label: "drawdown".to_string(),
        status,
    }
}

/// Builds the daily-trade readiness row from `DailyTradeRequirementSatisfied`.
fn daily_trade_row(events: &[StoredEvent]) -> Readiness {
    let satisfied = events
        .iter()
        .find(|e| matches!(e.event_type, AgentEvent::DailyTradeRequirementSatisfied));

    let status = match satisfied {
        Some(event) => {
            let trades = event
                .payload_json
                .get("trades")
                .and_then(|v| v.as_u64())
                .map(|n| n.to_string())
                .unwrap_or_else(|| "?".to_string());
            format!("satisfied ({trades} trades)")
        }
        None => "pending".to_string(),
    };

    Readiness {
        label: "daily trades".to_string(),
        status,
    }
}

/// Reads `max_total_drawdown_pct` and `kill_switch_drawdown_pct` from the
/// risk-policy JSON, falling back to defaults on any error.
fn load_thresholds(policy_path: &str) -> (f64, f64) {
    let parsed = std::fs::read_to_string(policy_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());

    let throttle = parsed
        .as_ref()
        .and_then(|v| v.get("max_total_drawdown_pct"))
        .and_then(Value::as_f64)
        .unwrap_or(DEFAULT_THROTTLE_PCT);

    let kill = parsed
        .as_ref()
        .and_then(|v| v.get("kill_switch_drawdown_pct"))
        .and_then(Value::as_f64)
        .unwrap_or(DEFAULT_KILL_PCT);

    (throttle, kill)
}

/// Normalizes a drawdown string into a percentage. Values at or below 1.0 are
/// treated as fractions (e.g. `0.0644` -> `6.44`); larger values are assumed to
/// already be percentages. Returns `None` for placeholders or unparsable input.
fn normalize_pct(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed == "—" {
        return None;
    }
    let value = trimmed.parse::<f64>().ok()?;
    if value.abs() <= 1.0 {
        Some(value * 100.0)
    } else {
        Some(value)
    }
}
