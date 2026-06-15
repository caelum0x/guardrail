//! Judge-facing submission scorecard endpoint.
//!
//! Aggregates configured judging sections with current report/event evidence.
//! Read-only and deterministic; it does not call other HTTP routes.

use axum::{extract::State, Json};
use event_store::AgentEvent;
use serde_json::{json, Value};

const CONFIG: &str = "configs/submission/scorecard.json";
const REPORT: &str = "data/run_report.json";

pub async fn scorecard(State(state): State<crate::routes::AppState>) -> Json<Value> {
    match build(&state) {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build(state: &crate::routes::AppState) -> anyhow::Result<Value> {
    let config: Value = serde_json::from_str(&std::fs::read_to_string(CONFIG)?)?;
    let report_path = std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| REPORT.to_string());
    let report = std::fs::read_to_string(&report_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
    let events = state.recent_events(500).unwrap_or_default();
    let tx_count = events
        .iter()
        .filter(|event| {
            matches!(event.event_type, AgentEvent::TxConfirmed)
                && (event.payload_json.get("tx_hash").is_some()
                    || event.payload_json.get("competition_tx").is_some())
        })
        .count();
    let daily_trade = events.iter().any(|event| {
        matches!(event.event_type, AgentEvent::DailyTradeRequirementSatisfied)
            || (matches!(event.event_type, AgentEvent::TxConfirmed)
                && event.payload_json.get("tx_hash").is_some())
    });
    let eligible_assets = market_data::Universe::load("configs/eligible_assets.bsc.json")
        .map(|universe| universe.enabled().len())
        .unwrap_or(0);
    let audit_ready = audit_artifacts_ready("configs/audit/export_manifest.json");
    let bnb_sdk_mapped = config_file_exists("configs/bnb/bnb_agent_sdk_map.json");
    let commerce_ready = config_file_exists("configs/bnb/erc8183_commerce.json");
    let skill_present = config_file_exists("skills/cmc-regime-routed-alpha/skill.yaml");
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
        "confirmed_txs": tx_count > 0,
        "registered": events.iter().any(|event| matches!(event.event_type, AgentEvent::TxConfirmed) && event.payload_json.get("competition_tx").is_some()),
        "daily_trade": daily_trade,
        "eligible_assets": eligible_assets > 0,
        "skill_present": skill_present,
        "twak_only": true,
        "bnb_sdk_mapped": bnb_sdk_mapped,
        "commerce_ready": commerce_ready,
        "audit_ready": audit_ready
    });

    let mut sections = Vec::new();
    let mut total_weight = 0f64;
    let mut earned_weight = 0f64;
    for section in config
        .get("sections")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let required = section
            .get("required_facts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let passed = required
            .iter()
            .filter(|name| {
                name.as_str()
                    .and_then(|key| facts.get(key))
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
            .count();
        let total = required.len();
        let weight = section.get("weight").and_then(Value::as_f64).unwrap_or(0.0);
        let pct = if total > 0 {
            passed as f64 / total as f64
        } else {
            1.0
        };
        total_weight += weight;
        earned_weight += weight * pct;
        sections.push(json!({
            "id": section.get("id").cloned().unwrap_or(json!("section")),
            "label": section.get("label").cloned().unwrap_or(json!("Section")),
            "weight": weight,
            "status": if passed == total { "ready" } else { "partial" },
            "passed_facts": passed,
            "total_facts": total,
            "score_pct": (pct * 100.0).round(),
            "evidence_routes": section.get("evidence_routes").cloned().unwrap_or(json!([])),
            "required_facts": required
        }));
    }
    let score_pct = if total_weight > 0.0 {
        earned_weight / total_weight * 100.0
    } else {
        0.0
    };
    let threshold = config
        .get("threshold_ready_pct")
        .and_then(Value::as_f64)
        .unwrap_or(85.0);
    Ok(json!({
        "config_path": CONFIG,
        "report_path": report_path,
        "name": config.get("name").cloned().unwrap_or(json!("Judge Scorecard")),
        "status": if score_pct >= threshold { "ready" } else { "partial" },
        "summary": {
            "score_pct": score_pct.round(),
            "threshold_ready_pct": threshold,
            "earned_weight": (earned_weight * 100.0).round() / 100.0,
            "total_weight": total_weight,
            "sections": sections.len()
        },
        "facts": facts,
        "sections": sections
    }))
}

fn config_file_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

fn audit_artifacts_ready(path: &str) -> bool {
    let Some(config) = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
    else {
        return false;
    };
    config
        .get("artifacts")
        .and_then(Value::as_array)
        .map(|items| {
            items.iter().all(|item| {
                !item
                    .get("required")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
                    || item
                        .get("path")
                        .and_then(Value::as_str)
                        .map(config_file_exists)
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}
