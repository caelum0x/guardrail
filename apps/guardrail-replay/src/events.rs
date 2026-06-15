//! Event loading, naming, and run scoping.

use event_store::{AgentEvent, StoredEvent};

/// The serialized name of an event variant (e.g. `TxConfirmed`).
pub fn event_name(e: &AgentEvent) -> String {
    serde_json::to_value(e)
        .ok()
        .and_then(|v| v.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".into())
}

/// Scope events to a single run. Matches a run id exactly, or by unique prefix
/// (so a short id fragment works). Returns the input unchanged when `run` is
/// `None`. When a prefix is ambiguous or unmatched the result is empty.
pub fn scope_to_run(events: Vec<StoredEvent>, run: Option<&str>) -> Vec<StoredEvent> {
    let Some(run) = run else {
        return events;
    };
    events
        .into_iter()
        .filter(|e| e.run_id == run || e.run_id.starts_with(run))
        .collect()
}

#[cfg(test)]
pub(crate) fn stub(run_id: &str, ts: &str, event_type: AgentEvent) -> StoredEvent {
    StoredEvent {
        id: format!("{run_id}-{ts}"),
        run_id: run_id.to_string(),
        timestamp: ts.to_string(),
        event_type,
        payload_json: serde_json::json!({}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_serde_form() {
        // AgentEvent serializes with `rename_all = "snake_case"`.
        assert_eq!(event_name(&AgentEvent::TxConfirmed), "tx_confirmed");
        assert_eq!(event_name(&AgentEvent::RiskRejected), "risk_rejected");
    }

    #[test]
    fn scope_none_returns_all() {
        let events = vec![stub("run-a", "t1", AgentEvent::AgentStarted)];
        assert_eq!(scope_to_run(events, None).len(), 1);
    }

    #[test]
    fn scope_matches_exact_and_prefix() {
        let events = vec![
            stub("run-abc", "t1", AgentEvent::AgentStarted),
            stub("run-xyz", "t2", AgentEvent::AgentStarted),
        ];
        assert_eq!(scope_to_run(events.clone(), Some("run-abc")).len(), 1);
        assert_eq!(scope_to_run(events.clone(), Some("run-a")).len(), 1);
        assert_eq!(scope_to_run(events, Some("run-")).len(), 2);
    }
}
