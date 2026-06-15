//! Event-log derived counters and latest-state extraction.

use std::path::Path;

use event_store::{AgentEvent, SqliteEventRepository, StoredEvent};

pub const SCAN_LIMIT: usize = 10_000;

/// Counters and latest-state derived from the event log.
#[derive(Default)]
pub struct Counts {
    pub events: usize,
    pub proposed: usize,
    pub approved: usize,
    pub rejections: usize,
    pub clips: usize,
    pub quotes: usize,
    pub submitted: usize,
    pub trades: usize,
    pub reconciled: usize,
    pub daily_satisfied: usize,
    pub throttle_activations: usize,
    pub kill_switches: usize,
    /// Regime label from the most recent `RegimeClassified` event.
    pub latest_regime: Option<String>,
    /// Timestamp of the newest event (ISO-8601), if any.
    pub last_event_ts: Option<String>,
}

/// Tally counters from the event log. Returns zeros on any read error so the
/// exporter degrades gracefully rather than failing a scrape.
pub fn event_counts(db_path: &Path) -> Counts {
    // `recent` returns newest-first; the first RegimeClassified we see is latest.
    let events: Vec<StoredEvent> = SqliteEventRepository::open(db_path)
        .and_then(|repo| repo.recent(SCAN_LIMIT))
        .unwrap_or_default();

    let mut c = Counts {
        events: events.len(),
        last_event_ts: events.first().map(|e| e.timestamp.clone()),
        ..Default::default()
    };

    for e in &events {
        match e.event_type {
            AgentEvent::OrderProposed => c.proposed += 1,
            AgentEvent::RiskApproved => c.approved += 1,
            AgentEvent::RiskRejected => c.rejections += 1,
            AgentEvent::RiskClipped => c.clips += 1,
            AgentEvent::TwakQuoteReceived => c.quotes += 1,
            AgentEvent::TwakSwapSubmitted => c.submitted += 1,
            AgentEvent::TxConfirmed => c.trades += 1,
            AgentEvent::PortfolioReconciled => c.reconciled += 1,
            AgentEvent::DailyTradeRequirementSatisfied => c.daily_satisfied += 1,
            AgentEvent::DrawdownThrottleActivated => c.throttle_activations += 1,
            AgentEvent::KillSwitchTriggered => c.kill_switches += 1,
            AgentEvent::RegimeClassified if c.latest_regime.is_none() => {
                c.latest_regime = e
                    .payload_json
                    .get("regime")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            _ => {}
        }
    }
    c
}
