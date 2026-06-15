//! Submission prize map endpoint.
//!
//! Maps product evidence to hackathon prize/category claims using current run
//! facts and configured evidence paths.

use axum::extract::State;
use axum::Json;
use event_store::AgentEvent;
use serde_json::{json, Value};

const PRIZE_MAP: &str = "configs/submission/prize_map.json";
const DEFAULT_REPORT: &str = "data/run_report.json";

pub async fn prizes(State(state): State<crate::routes::AppState>) -> Json<Value> {
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
    let events = state.recent_events(300).unwrap_or_default();
    let confirmed_txs = events
        .iter()
        .filter(|event| {
            matches!(event.event_type, AgentEvent::TxConfirmed)
                && event.payload_json.get("tx_hash").is_some()
        })
        .count();
    let daily_trade = events.iter().any(|event| {
        matches!(event.event_type, AgentEvent::DailyTradeRequirementSatisfied)
            || (matches!(event.event_type, AgentEvent::TxConfirmed)
                && event.payload_json.get("tx_hash").is_some())
    });
    let facts = json!({
        "report_present": report.is_some(),
        "wallet_present": report
            .as_ref()
            .and_then(|value| value.get("wallet_address"))
            .and_then(Value::as_str)
            .map(|value| !value.is_empty())
            .unwrap_or(false),
        "policy_hash_present": report
            .as_ref()
            .and_then(|value| value.get("policy_hash"))
            .and_then(Value::as_str)
            .map(|value| !value.is_empty())
            .unwrap_or(false),
        "confirmed_txs": confirmed_txs > 0,
        "daily_trade": daily_trade
    });
    let configured: Value = serde_json::from_str(&std::fs::read_to_string(PRIZE_MAP)?)?;
    let mut rows = Vec::new();
    let mut ready = 0usize;
    for item in configured.as_array().cloned().unwrap_or_default() {
        let required = item
            .get("required_facts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let passed = required
            .iter()
            .filter(|key| {
                key.as_str()
                    .and_then(|name| facts.get(name))
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
            .count();
        let status = if passed == required.len() {
            "ready"
        } else {
            "partial"
        };
        if status == "ready" {
            ready += 1;
        }
        rows.push(json!({
            "id": item.get("id").cloned().unwrap_or(json!("unknown")),
            "label": item.get("label").cloned().unwrap_or(json!("Prize")),
            "claim": item.get("claim").cloned().unwrap_or(json!("")),
            "evidence_paths": item.get("evidence_paths").cloned().unwrap_or(json!([])),
            "required_facts": required,
            "passed_facts": passed,
            "total_facts": item.get("required_facts").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
            "status": status
        }));
    }
    Ok(json!({
        "path": PRIZE_MAP,
        "report_path": report_path,
        "facts": facts,
        "summary": {
            "categories": rows.len(),
            "ready": ready,
            "partial": rows.len().saturating_sub(ready)
        },
        "prizes": rows
    }))
}
