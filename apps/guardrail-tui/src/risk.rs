//! Risk panel model.
//!
//! Summarizes recent risk decisions from the event log: how many orders were
//! approved, clipped, or rejected, plus the last few human-readable rejection /
//! clip reasons. Reasons are drawn from event payloads where available and
//! degrade gracefully when absent.

use event_store::{AgentEvent, StoredEvent};

/// Maximum number of recent reasons to surface in the panel.
const MAX_REASONS: usize = 5;

/// Parsed view of recent risk activity.
#[derive(Debug, Clone)]
pub struct RiskPanel {
    pub available: bool,
    pub approved: usize,
    pub clipped: usize,
    pub rejected: usize,
    /// Most recent reasons (newest first), each prefixed with its decision kind.
    pub recent_reasons: Vec<String>,
}

impl RiskPanel {
    /// Placeholder used when the event log is unavailable.
    pub fn unavailable() -> Self {
        Self {
            available: false,
            approved: 0,
            clipped: 0,
            rejected: 0,
            recent_reasons: Vec::new(),
        }
    }

    /// Aggregates risk decisions from `events` (expected newest-first).
    pub fn from_events(events: &[StoredEvent]) -> Self {
        let mut approved = 0;
        let mut clipped = 0;
        let mut rejected = 0;
        let mut recent_reasons = Vec::new();

        for event in events {
            match event.event_type {
                AgentEvent::RiskApproved => approved += 1,
                AgentEvent::RiskClipped => {
                    clipped += 1;
                    if recent_reasons.len() < MAX_REASONS {
                        if let Some(reason) = clip_reason(event) {
                            recent_reasons.push(format!("clipped: {reason}"));
                        }
                    }
                }
                AgentEvent::RiskRejected => {
                    rejected += 1;
                    if recent_reasons.len() < MAX_REASONS {
                        for reason in reject_reasons(event) {
                            if recent_reasons.len() >= MAX_REASONS {
                                break;
                            }
                            recent_reasons.push(format!("rejected: {reason}"));
                        }
                    }
                }
                _ => {}
            }
        }

        Self {
            available: true,
            approved,
            clipped,
            rejected,
            recent_reasons,
        }
    }

    /// Total number of risk decisions observed.
    pub fn total(&self) -> usize {
        self.approved + self.clipped + self.rejected
    }
}

/// Extracts the human-readable reason(s) from a `RiskRejected` payload.
fn reject_reasons(event: &StoredEvent) -> Vec<String> {
    event
        .payload_json
        .get("reasons")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Derives a short reason for a `RiskClipped` payload. Clips typically only
/// carry the clipped amount, so we render that when present.
fn clip_reason(event: &StoredEvent) -> Option<String> {
    if let Some(amount) = event
        .payload_json
        .get("amount_usd")
        .and_then(|v| v.as_str())
    {
        let display = round_amount(amount);
        return Some(format!("order capped to ${display}"));
    }
    Some("position size capped".to_string())
}

/// Rounds a high-precision decimal amount string to two decimals for display,
/// falling back to the original string if it cannot be parsed.
fn round_amount(amount: &str) -> String {
    match amount.trim().parse::<f64>() {
        Ok(value) => format!("{value:.2}"),
        Err(_) => amount.to_string(),
    }
}
