//! Operator playbook endpoint.
//!
//! Selects the most relevant runbook from a small product-owned playbook file
//! using current report and event-log facts. Read-only.

use axum::extract::State;
use axum::Json;
use event_store::AgentEvent;
use serde_json::{json, Value};

const PLAYBOOKS: &str = "configs/playbooks/operator_actions.json";
const DEFAULT_REPORT: &str = "data/run_report.json";

pub async fn playbook(State(state): State<crate::routes::AppState>) -> Json<Value> {
    match build(&state) {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build(state: &crate::routes::AppState) -> anyhow::Result<Value> {
    let report_path = std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| DEFAULT_REPORT.into());
    let report = std::fs::read_to_string(&report_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
    let events = state.recent_events(200).unwrap_or_default();
    let playbooks: Value = serde_json::from_str(&std::fs::read_to_string(PLAYBOOKS)?)?;

    let report_present = report.is_some();
    let report_kill_switch = report
        .as_ref()
        .and_then(|value| value.get("kill_switch"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let event_kill_switch = events
        .iter()
        .any(|event| matches!(event.event_type, AgentEvent::KillSwitchTriggered));
    let tx_count = events
        .iter()
        .filter(|event| {
            matches!(event.event_type, AgentEvent::TxConfirmed)
                && event.payload_json.get("tx_hash").is_some()
        })
        .count();
    let risk_decisions = events
        .iter()
        .filter(|event| {
            matches!(
                event.event_type,
                AgentEvent::RiskApproved | AgentEvent::RiskRejected | AgentEvent::RiskClipped
            )
        })
        .count();

    let active_id = if report_kill_switch || event_kill_switch {
        "kill_switch"
    } else if !report_present {
        "bootstrap"
    } else if tx_count == 0 {
        "execution_proof"
    } else if risk_decisions == 0 {
        "operator_review"
    } else {
        "ready"
    };
    let active = playbooks
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("id").and_then(Value::as_str) == Some(active_id))
        })
        .cloned()
        .unwrap_or_else(|| json!({ "id": active_id, "commands": [] }));

    Ok(json!({
        "active_id": active_id,
        "active": active,
        "playbooks": playbooks,
        "facts": {
            "report_path": report_path,
            "report_present": report_present,
            "kill_switch": report_kill_switch || event_kill_switch,
            "events_visible": events.len(),
            "confirmed_txs": tx_count,
            "risk_decisions": risk_decisions
        }
    }))
}
