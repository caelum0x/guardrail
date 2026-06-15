//! Submission briefing endpoint.
//!
//! Packages current report facts, event evidence, and product-owned claims into
//! a concise judge/operator briefing. Read-only.

use axum::extract::State;
use axum::Json;
use event_store::AgentEvent;
use serde_json::{json, Value};

const BRIEFING_CONFIG: &str = "configs/briefings/submission_briefing.json";
const DEFAULT_REPORT: &str = "data/run_report.json";

pub async fn briefing(State(state): State<crate::routes::AppState>) -> Json<Value> {
    match build(&state) {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build(state: &crate::routes::AppState) -> anyhow::Result<Value> {
    let config: Value = serde_json::from_str(&std::fs::read_to_string(BRIEFING_CONFIG)?)?;
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
    let risk_decisions = events
        .iter()
        .filter(|event| {
            matches!(
                event.event_type,
                AgentEvent::RiskApproved | AgentEvent::RiskRejected | AgentEvent::RiskClipped
            )
        })
        .count();
    let daily_trade = events.iter().any(|event| {
        matches!(event.event_type, AgentEvent::DailyTradeRequirementSatisfied)
            || (matches!(event.event_type, AgentEvent::TxConfirmed)
                && event.payload_json.get("tx_hash").is_some())
    });
    let kill_switch = report
        .as_ref()
        .and_then(|value| value.get("kill_switch"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || events
            .iter()
            .any(|event| matches!(event.event_type, AgentEvent::KillSwitchTriggered));
    let status = if report.is_none() || kill_switch {
        "blocking"
    } else if confirmed_txs == 0 || !daily_trade {
        "needs_proof"
    } else {
        "ready"
    };

    Ok(json!({
        "status": status,
        "config_path": BRIEFING_CONFIG,
        "title": config.get("title").cloned().unwrap_or(json!("Submission Briefing")),
        "claims": config.get("claims").cloned().unwrap_or(json!([])),
        "artifact_paths": config.get("artifact_paths").cloned().unwrap_or(json!([])),
        "demo_commands": config.get("demo_commands").cloned().unwrap_or(json!([])),
        "facts": {
            "report_path": report_path,
            "report_present": report.is_some(),
            "run_id": report.as_ref().and_then(|value| value.get("run_id")).cloned(),
            "mode": report.as_ref().and_then(|value| value.get("mode")).cloned(),
            "nav_usd": report.as_ref().and_then(|value| value.get("nav_usd")).cloned(),
            "wallet_address": report.as_ref().and_then(|value| value.get("wallet_address")).cloned(),
            "policy_hash": report.as_ref().and_then(|value| value.get("policy_hash")).cloned(),
            "events_visible": events.len(),
            "confirmed_txs": confirmed_txs,
            "risk_decisions": risk_decisions,
            "daily_trade": daily_trade,
            "kill_switch": kill_switch
        }
    }))
}
