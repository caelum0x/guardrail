//! Pure check functions over a [`RunReport`].
//!
//! Each function is side-effect free: it inspects an immutable report (and a
//! few thresholds) and returns an optional [`MonitorAlert`]. This keeps the
//! logic trivially unit-testable and decoupled from IO/logging.

use rust_decimal::Decimal;

use crate::alerts::{MonitorAlert, Severity};
use crate::report::RunReport;

/// Parse a decimal-valued report field, treating empty/invalid input as `None`.
fn parse_decimal(raw: &str) -> Option<Decimal> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<Decimal>().ok()
}

/// Alert if the report has not been refreshed within `max_age_ms`.
///
/// Returns `None` when the report is fresh. A non-positive `max_age_ms`
/// disables the check.
pub fn report_is_stale(report: &RunReport, now_ms: i64, max_age_ms: i64) -> Option<MonitorAlert> {
    if max_age_ms <= 0 {
        return None;
    }
    let age_ms = now_ms.saturating_sub(report.updated_ms);
    if age_ms > max_age_ms {
        return Some(MonitorAlert::new(
            Severity::Warning,
            format!(
                "run report is stale: last update {age_ms} ms ago (max {max_age_ms} ms, run_id={})",
                report.run_id
            ),
        ));
    }
    None
}

/// Alert if drawdown breaches a soft (warning) or hard (critical) threshold.
///
/// `soft_pct` and `hard_pct` are expressed as positive percentages (e.g. `10`
/// for 10%). The hard threshold takes precedence when both are breached. An
/// unparseable or absent drawdown field yields `None`.
pub fn drawdown_breach(
    report: &RunReport,
    soft_pct: Decimal,
    hard_pct: Decimal,
) -> Option<MonitorAlert> {
    let drawdown = parse_decimal(&report.total_drawdown_pct)?;
    // Normalize to a positive magnitude so callers may pass either sign.
    let magnitude = drawdown.abs();

    if magnitude >= hard_pct {
        return Some(MonitorAlert::new(
            Severity::Critical,
            format!(
                "drawdown {magnitude}% breached hard limit {hard_pct}% (run_id={})",
                report.run_id
            ),
        ));
    }
    if magnitude >= soft_pct {
        return Some(MonitorAlert::new(
            Severity::Warning,
            format!(
                "drawdown {magnitude}% breached soft limit {soft_pct}% (run_id={})",
                report.run_id
            ),
        ));
    }
    None
}

/// Alert (critical) if the agent's kill switch is engaged.
pub fn kill_switch_active(report: &RunReport) -> Option<MonitorAlert> {
    if report.kill_switch {
        return Some(MonitorAlert::new(
            Severity::Critical,
            format!("kill switch is ACTIVE (run_id={})", report.run_id),
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base_report() -> RunReport {
        RunReport {
            run_id: "run-1".to_string(),
            updated_ms: 1_000,
            ..RunReport::default()
        }
    }

    #[test]
    fn report_is_stale_flags_old_report() {
        let report = base_report();
        let alert = report_is_stale(&report, 11_000, 5_000).expect("should be stale");
        assert_eq!(alert.severity, Severity::Warning);
        assert!(alert.message.contains("stale"));
    }

    #[test]
    fn report_is_stale_passes_for_fresh_report() {
        let report = base_report();
        assert!(report_is_stale(&report, 3_000, 5_000).is_none());
    }

    #[test]
    fn report_is_stale_disabled_when_max_age_non_positive() {
        let report = base_report();
        assert!(report_is_stale(&report, 1_000_000, 0).is_none());
    }

    #[test]
    fn drawdown_breach_hard_takes_precedence() {
        let mut report = base_report();
        report.total_drawdown_pct = "25".to_string();
        let alert = drawdown_breach(&report, dec!(10), dec!(20)).expect("hard breach");
        assert_eq!(alert.severity, Severity::Critical);
        assert!(alert.message.contains("hard"));
    }

    #[test]
    fn drawdown_breach_soft_only() {
        let mut report = base_report();
        report.total_drawdown_pct = "12.5".to_string();
        let alert = drawdown_breach(&report, dec!(10), dec!(20)).expect("soft breach");
        assert_eq!(alert.severity, Severity::Warning);
        assert!(alert.message.contains("soft"));
    }

    #[test]
    fn drawdown_breach_handles_negative_sign() {
        let mut report = base_report();
        report.total_drawdown_pct = "-22".to_string();
        let alert = drawdown_breach(&report, dec!(10), dec!(20)).expect("hard breach");
        assert_eq!(alert.severity, Severity::Critical);
    }

    #[test]
    fn drawdown_breach_none_when_within_limits() {
        let mut report = base_report();
        report.total_drawdown_pct = "5".to_string();
        assert!(drawdown_breach(&report, dec!(10), dec!(20)).is_none());
    }

    #[test]
    fn drawdown_breach_none_when_unparseable() {
        let mut report = base_report();
        report.total_drawdown_pct = "".to_string();
        assert!(drawdown_breach(&report, dec!(10), dec!(20)).is_none());
    }

    #[test]
    fn kill_switch_active_flags_when_set() {
        let mut report = base_report();
        report.kill_switch = true;
        let alert = kill_switch_active(&report).expect("kill switch alert");
        assert_eq!(alert.severity, Severity::Critical);
    }

    #[test]
    fn kill_switch_active_none_when_unset() {
        let report = base_report();
        assert!(kill_switch_active(&report).is_none());
    }
}
