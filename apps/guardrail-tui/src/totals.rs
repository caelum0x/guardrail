//! Aggregated event counts derived from the event store.

use std::collections::BTreeMap;

use event_store::{AgentEvent, StoredEvent};

/// Counts of events grouped by their (snake_case) event-type name, plus a flag
/// indicating whether the underlying store was reachable.
#[derive(Debug, Clone)]
pub struct EventTotals {
    pub available: bool,
    pub total: usize,
    /// Ordered map of event-type name -> count (BTreeMap keeps display stable).
    pub by_type: BTreeMap<String, usize>,
}

impl EventTotals {
    /// Totals for the case where the database is missing or unreadable.
    pub fn unavailable() -> Self {
        Self {
            available: false,
            total: 0,
            by_type: BTreeMap::new(),
        }
    }

    /// Aggregates optionally-loaded events, treating `None` (an unreachable
    /// store) as unavailable and `Some` as the available event window.
    pub fn from_recent(events: &Option<Vec<StoredEvent>>) -> Self {
        match events {
            Some(events) => Self::from_events(events),
            None => Self::unavailable(),
        }
    }

    /// Aggregates the supplied events into per-type counts.
    pub fn from_events(events: &[StoredEvent]) -> Self {
        let mut by_type: BTreeMap<String, usize> = BTreeMap::new();
        for event in events {
            let name = event_type_name(&event.event_type);
            *by_type.entry(name).or_insert(0) += 1;
        }
        Self {
            available: true,
            total: events.len(),
            by_type,
        }
    }

    /// Convenience accessor for a single event type's count.
    pub fn count(&self, name: &str) -> usize {
        self.by_type.get(name).copied().unwrap_or(0)
    }

    /// Confirmed trades (`TxConfirmed`).
    pub fn trades(&self) -> usize {
        self.count("tx_confirmed")
    }

    /// Risk rejections (`RiskRejected`).
    pub fn rejections(&self) -> usize {
        self.count("risk_rejected")
    }
}

/// Maps an [`AgentEvent`] to its snake_case name without panicking. Falls back
/// to a placeholder if serialization ever fails to produce a string.
fn event_type_name(event_type: &AgentEvent) -> String {
    match serde_json::to_value(event_type) {
        Ok(serde_json::Value::String(name)) => name,
        _ => "unknown".to_string(),
    }
}
