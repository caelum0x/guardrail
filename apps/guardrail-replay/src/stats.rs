//! Lifecycle statistics derived from the event log.

use std::collections::BTreeMap;

use event_store::{AgentEvent, StoredEvent};
use serde::Serialize;

use crate::events::event_name;

/// The strategy → risk → execution funnel plus safety-event counts.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Funnel {
    pub proposed: usize,
    pub approved: usize,
    pub rejected: usize,
    pub clipped: usize,
    pub quotes: usize,
    pub submitted: usize,
    pub confirmed: usize,
    pub reconciled: usize,
    pub daily_satisfied: usize,
    pub throttle_activations: usize,
    pub kill_switches: usize,
}

/// Tally the funnel from a slice of events.
pub fn funnel(events: &[StoredEvent]) -> Funnel {
    let mut f = Funnel::default();
    for e in events {
        match e.event_type {
            AgentEvent::OrderProposed => f.proposed += 1,
            AgentEvent::RiskApproved => f.approved += 1,
            AgentEvent::RiskRejected => f.rejected += 1,
            AgentEvent::RiskClipped => f.clipped += 1,
            AgentEvent::TwakQuoteReceived => f.quotes += 1,
            AgentEvent::TwakSwapSubmitted => f.submitted += 1,
            AgentEvent::TxConfirmed => f.confirmed += 1,
            AgentEvent::PortfolioReconciled => f.reconciled += 1,
            AgentEvent::DailyTradeRequirementSatisfied => f.daily_satisfied += 1,
            AgentEvent::DrawdownThrottleActivated => f.throttle_activations += 1,
            AgentEvent::KillSwitchTriggered => f.kill_switches += 1,
            _ => {}
        }
    }
    f
}

impl Funnel {
    /// Approval rate over all orders that reached the risk gate (approved +
    /// rejected). `None` when no orders were gated.
    pub fn approval_rate(&self) -> Option<f64> {
        let gated = self.approved + self.rejected;
        (gated > 0).then(|| self.approved as f64 / gated as f64)
    }

    /// Fill rate: confirmed swaps over submitted swaps. `None` when none submitted.
    pub fn fill_rate(&self) -> Option<f64> {
        (self.submitted > 0).then(|| self.confirmed as f64 / self.submitted as f64)
    }
}

/// Coverage of the event stream: distinct runs and the first/last timestamps.
#[derive(Debug, Default, Serialize)]
pub struct Span {
    pub total_events: usize,
    pub runs: usize,
    pub first_timestamp: Option<String>,
    pub last_timestamp: Option<String>,
}

/// Compute the span over events. Assumes events are sorted oldest-first; if not,
/// it still reports the min/max lexicographically (ISO-8601 timestamps sort
/// chronologically).
pub fn span(events: &[StoredEvent]) -> Span {
    let runs = events
        .iter()
        .map(|e| e.run_id.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let first = events.iter().map(|e| &e.timestamp).min().cloned();
    let last = events.iter().map(|e| &e.timestamp).max().cloned();
    Span {
        total_events: events.len(),
        runs,
        first_timestamp: first,
        last_timestamp: last,
    }
}

/// Per-event-type counts, ordered by name.
pub fn type_counts(events: &[StoredEvent]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for e in events {
        *counts.entry(event_name(&e.event_type)).or_default() += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::stub;

    fn sample() -> Vec<StoredEvent> {
        vec![
            stub("r1", "2026-01-01T00:00:00Z", AgentEvent::OrderProposed),
            stub("r1", "2026-01-01T00:00:01Z", AgentEvent::RiskApproved),
            stub("r1", "2026-01-01T00:00:02Z", AgentEvent::TwakSwapSubmitted),
            stub("r1", "2026-01-01T00:00:03Z", AgentEvent::TxConfirmed),
            stub("r1", "2026-01-01T00:00:04Z", AgentEvent::OrderProposed),
            stub("r2", "2026-01-02T00:00:00Z", AgentEvent::RiskRejected),
        ]
    }

    #[test]
    fn funnel_counts_each_stage() {
        let f = funnel(&sample());
        assert_eq!(f.proposed, 2);
        assert_eq!(f.approved, 1);
        assert_eq!(f.rejected, 1);
        assert_eq!(f.submitted, 1);
        assert_eq!(f.confirmed, 1);
    }

    #[test]
    fn rates_are_ratios() {
        let f = funnel(&sample());
        assert_eq!(f.approval_rate(), Some(0.5));
        assert_eq!(f.fill_rate(), Some(1.0));
    }

    #[test]
    fn rates_are_none_when_empty() {
        let f = Funnel::default();
        assert_eq!(f.approval_rate(), None);
        assert_eq!(f.fill_rate(), None);
    }

    #[test]
    fn span_reports_runs_and_bounds() {
        let s = span(&sample());
        assert_eq!(s.total_events, 6);
        assert_eq!(s.runs, 2);
        assert_eq!(s.first_timestamp.as_deref(), Some("2026-01-01T00:00:00Z"));
        assert_eq!(s.last_timestamp.as_deref(), Some("2026-01-02T00:00:00Z"));
    }
}
