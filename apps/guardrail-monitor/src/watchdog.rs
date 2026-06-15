//! Loads the agent run report and evaluates the pure checks against it.

use std::path::{Path, PathBuf};

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::alerts::MonitorAlert;
use crate::checks::{drawdown_breach, kill_switch_active, report_is_stale};
use crate::report::RunReport;

/// Environment variable overriding the report path.
const REPORT_PATH_ENV: &str = "GUARDRAIL_REPORT";
/// Default location of the run report relative to the working directory.
const DEFAULT_REPORT_PATH: &str = "data/run_report.json";

/// Maximum acceptable report age before it is considered stale (60s).
const MAX_AGE_MS: i64 = 60_000;
/// Soft drawdown threshold (warning) as a percentage.
fn soft_drawdown_pct() -> Decimal {
    dec!(10)
}
/// Hard drawdown threshold (critical) as a percentage.
fn hard_drawdown_pct() -> Decimal {
    dec!(20)
}

/// Resolve the report path from the environment or fall back to the default.
fn report_path() -> PathBuf {
    match std::env::var(REPORT_PATH_ENV) {
        Ok(value) if !value.trim().is_empty() => PathBuf::from(value),
        _ => PathBuf::from(DEFAULT_REPORT_PATH),
    }
}

/// Read and parse the run report at `path`.
///
/// Returns `Ok(None)` if the file is absent (a non-fatal condition the caller
/// is expected to warn about), and an error only for read or parse failures.
fn load_report(path: &Path) -> anyhow::Result<Option<RunReport>> {
    if !path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
    let report: RunReport = serde_json::from_str(&contents)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", path.display()))?;
    Ok(Some(report))
}

/// Evaluate all checks against a report, collecting any raised alerts.
///
/// Pure: no IO or logging, so it can be unit tested directly.
pub fn evaluate(report: &RunReport, now_ms: i64) -> Vec<MonitorAlert> {
    [
        report_is_stale(report, now_ms, MAX_AGE_MS),
        drawdown_breach(report, soft_drawdown_pct(), hard_drawdown_pct()),
        kill_switch_active(report),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Outcome of a single watchdog cycle.
///
/// Carries the `run_id` of the inspected report and any alerts raised so the
/// caller can take side-effecting follow-up actions (such as posting to a
/// webhook) without re-reading the report.
#[derive(Debug, Clone, Default)]
pub struct CycleOutcome {
    /// The report's run identifier, or empty when no report was available.
    pub run_id: String,
    /// Alerts raised during the cycle (empty when clear or no report).
    pub alerts: Vec<MonitorAlert>,
}

/// Run a single watchdog cycle: load the report, evaluate checks, log alerts.
///
/// A missing report logs a warning and returns an empty outcome so the monitor
/// keeps running. Read/parse errors are logged but also swallowed to keep the
/// loop resilient. The returned [`CycleOutcome`] lets callers act on the
/// raised alerts (e.g. outbound webhook alerting).
pub fn run_once(now_ms: i64) -> CycleOutcome {
    let path = report_path();
    match load_report(&path) {
        Ok(None) => {
            tracing::warn!(path = %path.display(), "run report not found; skipping cycle");
            CycleOutcome::default()
        }
        Ok(Some(report)) => {
            let alerts = evaluate(&report, now_ms);
            if alerts.is_empty() {
                tracing::info!(
                    run_id = %report.run_id,
                    mode = %report.mode,
                    regime = %report.regime,
                    "watchdog clear: no alerts"
                );
            } else {
                for alert in &alerts {
                    match alert.severity {
                        crate::alerts::Severity::Critical => {
                            tracing::error!(target: "watchdog", "{}", alert.format())
                        }
                        crate::alerts::Severity::Warning => {
                            tracing::warn!(target: "watchdog", "{}", alert.format())
                        }
                        crate::alerts::Severity::Info => {
                            tracing::info!(target: "watchdog", "{}", alert.format())
                        }
                    }
                }
            }
            CycleOutcome {
                run_id: report.run_id,
                alerts,
            }
        }
        Err(e) => {
            tracing::error!(path = %path.display(), error = %e, "failed to load run report");
            CycleOutcome::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alerts::Severity;

    fn report_with_drawdown(drawdown: &str, kill: bool, updated_ms: i64) -> RunReport {
        RunReport {
            run_id: "r".to_string(),
            updated_ms,
            total_drawdown_pct: drawdown.to_string(),
            kill_switch: kill,
            ..RunReport::default()
        }
    }

    #[test]
    fn evaluate_returns_no_alerts_for_healthy_report() {
        let report = report_with_drawdown("3", false, 1_000);
        let alerts = evaluate(&report, 2_000);
        assert!(alerts.is_empty());
    }

    #[test]
    fn evaluate_collects_multiple_alerts() {
        // Stale (age 100s > 60s), hard drawdown breach, and kill switch.
        let report = report_with_drawdown("30", true, 0);
        let alerts = evaluate(&report, 100_000);
        assert_eq!(alerts.len(), 3);
        assert!(alerts
            .iter()
            .all(|a| a.severity == Severity::Critical || a.severity == Severity::Warning));
    }
}
