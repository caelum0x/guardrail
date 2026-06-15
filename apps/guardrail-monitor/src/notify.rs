//! Outbound webhook alerting for the watchdog.
//!
//! Builds [`notifier::Notification`]s from the cycle's [`MonitorAlert`]s and
//! delivers them via the reusable [`notifier`] crate. Network failures are
//! surfaced as errors for the caller to log; they must never abort the watchdog
//! loop.

use notifier::{ConsoleSink, Notification, Sink, WebhookSink};

use crate::alerts::MonitorAlert;

/// Stable source label attached to every notification emitted by the monitor.
const SOURCE: &str = "guardrail-monitor";

/// Convert the cycle's alerts into transport-agnostic notifications.
///
/// The `run_id` is folded into each message so downstream sinks retain the
/// context that the previous bespoke payload carried in a dedicated field.
fn to_notifications(run_id: &str, alerts: &[MonitorAlert]) -> Vec<Notification> {
    alerts
        .iter()
        .map(|alert| {
            Notification::new(
                SOURCE,
                alert.severity.label(),
                format!("[run {run_id}] {}", alert.message),
            )
        })
        .collect()
}

/// Log the cycle's alerts locally via [`notifier::ConsoleSink`].
///
/// This never performs any network I/O and is infallible from the caller's
/// perspective, so it is safe to invoke on every alertable cycle.
pub async fn log_alerts(run_id: &str, alerts: &[MonitorAlert]) {
    let notes = to_notifications(run_id, alerts);
    let sink = ConsoleSink::new();
    if let Err(e) = sink.deliver(&notes).await {
        tracing::warn!(run_id = %run_id, error = %e, "failed to log alerts locally");
    }
}

/// POST the cycle's alerts to `webhook_url` via [`notifier::WebhookSink`].
///
/// Returns an error on transport failures or a non-success HTTP status. The
/// caller is expected to log the error rather than propagate it, so the
/// watchdog loop stays resilient.
pub async fn post_alerts(
    webhook_url: &str,
    run_id: &str,
    alerts: &[MonitorAlert],
) -> anyhow::Result<()> {
    let notes = to_notifications(run_id, alerts);
    let sink = WebhookSink::new(webhook_url)?;
    sink.deliver(&notes).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alerts::Severity;

    #[test]
    fn builds_notifications_with_run_context() {
        let alerts = vec![
            MonitorAlert::new(Severity::Warning, "report is stale"),
            MonitorAlert::new(Severity::Critical, "kill switch engaged"),
        ];
        let notes = to_notifications("run-42", &alerts);
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].source, SOURCE);
        assert_eq!(notes[0].severity, "WARNING");
        assert!(notes[0].message.contains("run-42"));
        assert!(notes[0].message.contains("report is stale"));
        assert_eq!(notes[1].severity, "CRITICAL");
    }
}
