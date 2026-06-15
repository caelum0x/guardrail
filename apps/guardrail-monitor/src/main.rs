//! guardrail-monitor: a watchdog that periodically inspects the agent's run
//! report and raises alerts for staleness, drawdown breaches, and an engaged
//! kill switch.

mod alerts;
mod checks;
mod notify;
mod report;
mod watchdog;

use std::time::Duration;

use crate::alerts::Severity;
use crate::watchdog::CycleOutcome;

/// Environment variable controlling how many watchdog cycles to run.
const CHECKS_ENV: &str = "GUARDRAIL_MONITOR_CHECKS";
/// Environment variable holding the outbound alert webhook URL.
const WEBHOOK_ENV: &str = "GUARDRAIL_WEBHOOK";
/// Default number of watchdog cycles before exiting.
const DEFAULT_CHECKS: u32 = 3;
/// Delay between watchdog cycles.
const CYCLE_INTERVAL: Duration = Duration::from_secs(5);

/// Resolve the configured number of check iterations, defaulting on bad input.
fn check_iterations() -> u32 {
    std::env::var(CHECKS_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(DEFAULT_CHECKS)
}

/// Current wall-clock time in epoch milliseconds.
fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Resolve the configured webhook URL, treating empty values as unset.
fn webhook_url() -> Option<String> {
    match std::env::var(WEBHOOK_ENV) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

/// Dispatch the cycle's alerts to local logging and, when configured, a webhook.
///
/// Warning-or-higher alerts are always logged locally via [`notifier::ConsoleSink`].
/// They are additionally posted to the webhook when a URL is set. Network failures
/// are logged and swallowed so the watchdog loop is never aborted by webhook
/// problems, and no network call is made when no webhook is configured.
async fn dispatch_webhook(webhook_url: Option<&str>, outcome: &CycleOutcome) {
    let alertable: Vec<_> = outcome
        .alerts
        .iter()
        .filter(|alert| alert.severity >= Severity::Warning)
        .cloned()
        .collect();
    if alertable.is_empty() {
        return;
    }

    notify::log_alerts(&outcome.run_id, &alertable).await;

    let Some(url) = webhook_url else {
        return;
    };
    match notify::post_alerts(url, &outcome.run_id, &alertable).await {
        Ok(()) => tracing::info!(
            run_id = %outcome.run_id,
            count = alertable.len(),
            "posted alerts to webhook"
        ),
        Err(e) => tracing::warn!(
            run_id = %outcome.run_id,
            error = %e,
            "failed to post alerts to webhook"
        ),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    observability::tracing_setup::init();

    let iterations = check_iterations();
    let webhook = webhook_url();
    tracing::info!(
        iterations,
        webhook_configured = webhook.is_some(),
        "guardrail-monitor starting"
    );

    if iterations == 0 {
        let mut cycle = 0u64;
        loop {
            cycle += 1;
            tracing::info!(cycle, "running watchdog cycle");
            let outcome = watchdog::run_once(now_ms());
            dispatch_webhook(webhook.as_deref(), &outcome).await;
            tokio::time::sleep(CYCLE_INTERVAL).await;
        }
    }

    for cycle in 1..=iterations {
        tracing::info!(cycle, iterations, "running watchdog cycle");
        let outcome = watchdog::run_once(now_ms());
        dispatch_webhook(webhook.as_deref(), &outcome).await;
        if cycle < iterations {
            tokio::time::sleep(CYCLE_INTERVAL).await;
        }
    }

    tracing::info!("guardrail-monitor finished");
    Ok(())
}
