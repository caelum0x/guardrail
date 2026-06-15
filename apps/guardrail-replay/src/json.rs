//! JSON output mode for commands that support `--json`.

use event_store::{AgentEvent, StoredEvent};
use serde_json::{json, Value};

use crate::cli::Command;
use crate::events::event_name;
use crate::stats::{funnel, span, type_counts};

/// Build the JSON document for a command over `events`. Returns `None` for
/// commands that have no JSON form (CSV export writes its own output).
pub fn render(command: &Command, events: &[StoredEvent]) -> Option<Value> {
    match command {
        Command::Journal => Some(json!({ "events": events_to_json(events) })),
        Command::Trades => Some(json!({ "trades": trades_to_json(events) })),
        Command::Risk => Some(json!({ "risk_events": risk_to_json(events) })),
        Command::Summary => Some(json!({ "counts": type_counts(events) })),
        Command::Stats => Some(json!({ "funnel": funnel(events), "span": span(events) })),
        Command::Runs => Some(json!({ "runs": runs_to_json(events) })),
        Command::ExportCsv { .. } => None,
    }
}

fn event_to_json(e: &StoredEvent) -> Value {
    json!({
        "id": e.id,
        "run_id": e.run_id,
        "timestamp": e.timestamp,
        "type": event_name(&e.event_type),
        "payload": e.payload_json,
    })
}

fn events_to_json(events: &[StoredEvent]) -> Vec<Value> {
    events.iter().map(event_to_json).collect()
}

fn trades_to_json(events: &[StoredEvent]) -> Vec<Value> {
    events
        .iter()
        .filter(|e| matches!(e.event_type, AgentEvent::TxConfirmed))
        .map(event_to_json)
        .collect()
}

fn risk_to_json(events: &[StoredEvent]) -> Vec<Value> {
    events
        .iter()
        .filter(|e| matches!(e.event_type, AgentEvent::RiskRejected | AgentEvent::RiskClipped))
        .map(event_to_json)
        .collect()
}

fn runs_to_json(events: &[StoredEvent]) -> Vec<Value> {
    use std::collections::BTreeMap;
    let mut by_run: BTreeMap<&str, (usize, String, String)> = BTreeMap::new();
    for e in events {
        let entry = by_run
            .entry(e.run_id.as_str())
            .or_insert((0, e.timestamp.clone(), e.timestamp.clone()));
        entry.0 += 1;
        if e.timestamp < entry.1 {
            entry.1 = e.timestamp.clone();
        }
        if e.timestamp > entry.2 {
            entry.2 = e.timestamp.clone();
        }
    }
    by_run
        .into_iter()
        .map(|(run, (count, first, last))| {
            json!({ "run_id": run, "events": count, "first": first, "last": last })
        })
        .collect()
}
