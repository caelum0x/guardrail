//! Typed alerts and pure threshold-evaluation functions.
//!
//! Each evaluator is a pure function of its inputs and returns zero or more
//! [`Alert`]s. They never log, mutate global state, or perform I/O, which makes
//! them trivial to unit test and safe to call from any context.

use serde::{Deserialize, Serialize};

/// The set of alert conditions monitored by the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertKind {
    /// Drawdown crossed the soft warning limit.
    DrawdownSoft,
    /// Drawdown crossed the hard stop limit.
    DrawdownHard,
    /// Market data is older than the freshness budget.
    DataStale,
    /// Realized slippage exceeded the tolerated bound.
    SlippageHigh,
    /// Internal vs. broker reconciliation disagrees.
    ReconMismatch,
    /// The kill switch has been engaged.
    KillSwitch,
    /// An expected daily trade did not occur.
    DailyTradeMissing,
}

/// Severity of an alert, ordered from least to most urgent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Informational; no action required.
    Info,
    /// Warning; a human should look soon.
    Warning,
    /// Critical; trading should pause or stop.
    Critical,
}

/// A single fired alert with its severity and a human-readable message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Alert {
    /// The condition that fired.
    pub kind: AlertKind,
    /// How urgent the condition is.
    pub severity: Severity,
    /// Human-readable detail for operators.
    pub message: String,
}

impl Alert {
    /// Construct an alert with an owned message.
    pub fn new(kind: AlertKind, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            kind,
            severity,
            message: message.into(),
        }
    }
}

/// Thresholds that drive alert evaluation. All fractions are expressed as
/// positive ratios (for example `0.05` for five percent).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Drawdown fraction at which a soft warning fires.
    pub drawdown_soft: f64,
    /// Drawdown fraction at which a hard stop fires.
    pub drawdown_hard: f64,
    /// Maximum tolerated market-data age in seconds.
    pub data_max_age_secs: u64,
    /// Slippage fraction above which an alert fires.
    pub slippage_max: f64,
    /// Reconciliation difference above which a mismatch fires.
    pub recon_max_diff: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            drawdown_soft: 0.05,
            drawdown_hard: 0.10,
            data_max_age_secs: 120,
            slippage_max: 0.005,
            recon_max_diff: 0.01,
        }
    }
}

/// Evaluate drawdown against the soft and hard limits.
///
/// `drawdown` is a positive fraction of peak equity lost (for example `0.06`
/// means a six percent drawdown). Returns the most severe applicable alert.
pub fn evaluate_drawdown(drawdown: f64, thresholds: &AlertThresholds) -> Vec<Alert> {
    if drawdown >= thresholds.drawdown_hard {
        vec![Alert::new(
            AlertKind::DrawdownHard,
            Severity::Critical,
            format!(
                "drawdown {:.2}% >= hard limit {:.2}%",
                drawdown * 100.0,
                thresholds.drawdown_hard * 100.0
            ),
        )]
    } else if drawdown >= thresholds.drawdown_soft {
        vec![Alert::new(
            AlertKind::DrawdownSoft,
            Severity::Warning,
            format!(
                "drawdown {:.2}% >= soft limit {:.2}%",
                drawdown * 100.0,
                thresholds.drawdown_soft * 100.0
            ),
        )]
    } else {
        Vec::new()
    }
}

/// Evaluate market-data freshness.
pub fn evaluate_data_age(age_secs: u64, thresholds: &AlertThresholds) -> Vec<Alert> {
    if age_secs > thresholds.data_max_age_secs {
        vec![Alert::new(
            AlertKind::DataStale,
            Severity::Critical,
            format!(
                "market data age {}s > max {}s",
                age_secs, thresholds.data_max_age_secs
            ),
        )]
    } else {
        Vec::new()
    }
}

/// Evaluate realized slippage against tolerance.
pub fn evaluate_slippage(slippage: f64, thresholds: &AlertThresholds) -> Vec<Alert> {
    if slippage.abs() > thresholds.slippage_max {
        vec![Alert::new(
            AlertKind::SlippageHigh,
            Severity::Warning,
            format!(
                "slippage {:.4} exceeds max {:.4}",
                slippage, thresholds.slippage_max
            ),
        )]
    } else {
        Vec::new()
    }
}

/// Evaluate a reconciliation difference between internal and broker state.
pub fn evaluate_reconciliation(diff: f64, thresholds: &AlertThresholds) -> Vec<Alert> {
    if diff.abs() > thresholds.recon_max_diff {
        vec![Alert::new(
            AlertKind::ReconMismatch,
            Severity::Critical,
            format!(
                "reconciliation diff {:.4} exceeds max {:.4}",
                diff, thresholds.recon_max_diff
            ),
        )]
    } else {
        Vec::new()
    }
}

/// Emit a kill-switch alert when the switch is engaged.
pub fn evaluate_kill_switch(engaged: bool) -> Vec<Alert> {
    if engaged {
        vec![Alert::new(
            AlertKind::KillSwitch,
            Severity::Critical,
            "kill switch engaged",
        )]
    } else {
        Vec::new()
    }
}

/// Emit an alert when an expected daily trade did not happen.
pub fn evaluate_daily_trade(trade_executed: bool) -> Vec<Alert> {
    if trade_executed {
        Vec::new()
    } else {
        vec![Alert::new(
            AlertKind::DailyTradeMissing,
            Severity::Warning,
            "expected daily trade did not execute",
        )]
    }
}

/// A bundle of all observable inputs, evaluated together.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlertInputs {
    /// Positive drawdown fraction of peak equity.
    pub drawdown: f64,
    /// Age of the most recent market data in seconds.
    pub data_age_secs: u64,
    /// Most recent realized slippage fraction.
    pub slippage: f64,
    /// Reconciliation difference between internal and broker state.
    pub recon_diff: f64,
    /// Whether the kill switch is engaged.
    pub kill_switch: bool,
    /// Whether the expected daily trade executed.
    pub daily_trade_executed: bool,
}

/// Evaluate every condition and collect all fired alerts.
pub fn evaluate_all(inputs: &AlertInputs, thresholds: &AlertThresholds) -> Vec<Alert> {
    let mut alerts = Vec::new();
    alerts.extend(evaluate_drawdown(inputs.drawdown, thresholds));
    alerts.extend(evaluate_data_age(inputs.data_age_secs, thresholds));
    alerts.extend(evaluate_slippage(inputs.slippage, thresholds));
    alerts.extend(evaluate_reconciliation(inputs.recon_diff, thresholds));
    alerts.extend(evaluate_kill_switch(inputs.kill_switch));
    alerts.extend(evaluate_daily_trade(inputs.daily_trade_executed));
    alerts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thresholds() -> AlertThresholds {
        AlertThresholds::default()
    }

    #[test]
    fn drawdown_below_soft_is_quiet() {
        assert!(evaluate_drawdown(0.01, &thresholds()).is_empty());
    }

    #[test]
    fn drawdown_at_soft_warns() {
        let alerts = evaluate_drawdown(0.05, &thresholds());
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].kind, AlertKind::DrawdownSoft);
        assert_eq!(alerts[0].severity, Severity::Warning);
    }

    #[test]
    fn drawdown_at_hard_is_critical_and_not_doubled() {
        let alerts = evaluate_drawdown(0.10, &thresholds());
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].kind, AlertKind::DrawdownHard);
        assert_eq!(alerts[0].severity, Severity::Critical);
    }

    #[test]
    fn data_age_within_budget_is_quiet() {
        assert!(evaluate_data_age(120, &thresholds()).is_empty());
    }

    #[test]
    fn data_age_over_budget_is_stale() {
        let alerts = evaluate_data_age(121, &thresholds());
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].kind, AlertKind::DataStale);
    }

    #[test]
    fn slippage_within_tolerance_is_quiet() {
        assert!(evaluate_slippage(0.004, &thresholds()).is_empty());
    }

    #[test]
    fn slippage_over_tolerance_fires() {
        let alerts = evaluate_slippage(0.006, &thresholds());
        assert_eq!(alerts[0].kind, AlertKind::SlippageHigh);
    }

    #[test]
    fn reconciliation_uses_absolute_difference() {
        let alerts = evaluate_reconciliation(-0.05, &thresholds());
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].kind, AlertKind::ReconMismatch);
    }

    #[test]
    fn kill_switch_fires_only_when_engaged() {
        assert!(evaluate_kill_switch(false).is_empty());
        let alerts = evaluate_kill_switch(true);
        assert_eq!(alerts[0].kind, AlertKind::KillSwitch);
        assert_eq!(alerts[0].severity, Severity::Critical);
    }

    #[test]
    fn daily_trade_missing_fires_when_not_executed() {
        assert!(evaluate_daily_trade(true).is_empty());
        let alerts = evaluate_daily_trade(false);
        assert_eq!(alerts[0].kind, AlertKind::DailyTradeMissing);
    }

    #[test]
    fn evaluate_all_collects_every_fired_alert() {
        let inputs = AlertInputs {
            drawdown: 0.12,
            data_age_secs: 1000,
            slippage: 0.02,
            recon_diff: 0.5,
            kill_switch: true,
            daily_trade_executed: false,
        };
        let alerts = evaluate_all(&inputs, &thresholds());
        assert_eq!(alerts.len(), 6);
        assert!(alerts.iter().any(|a| a.kind == AlertKind::DrawdownHard));
        assert!(alerts.iter().any(|a| a.kind == AlertKind::DataStale));
        assert!(alerts.iter().any(|a| a.kind == AlertKind::SlippageHigh));
        assert!(alerts.iter().any(|a| a.kind == AlertKind::ReconMismatch));
        assert!(alerts.iter().any(|a| a.kind == AlertKind::KillSwitch));
        assert!(alerts
            .iter()
            .any(|a| a.kind == AlertKind::DailyTradeMissing));
    }

    #[test]
    fn evaluate_all_is_quiet_when_healthy() {
        let inputs = AlertInputs {
            drawdown: 0.0,
            data_age_secs: 1,
            slippage: 0.0,
            recon_diff: 0.0,
            kill_switch: false,
            daily_trade_executed: true,
        };
        assert!(evaluate_all(&inputs, &thresholds()).is_empty());
    }
}
