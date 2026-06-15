//! ERC-8183 commerce readiness endpoint.
//!
//! Surfaces how Guardrail maps the BNB Agent SDK agentic-commerce lifecycle to
//! its read-only deliverables. No job is created and no escrow is touched.

use axum::Json;
use serde_json::{json, Value};

const CONFIG: &str = "configs/bnb/erc8183_commerce.json";

pub async fn commerce() -> Json<Value> {
    match build() {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build() -> anyhow::Result<Value> {
    let config: Value = serde_json::from_str(&std::fs::read_to_string(CONFIG)?)?;
    let report_path = config
        .get("report_path")
        .and_then(Value::as_str)
        .unwrap_or("data/run_report.json");
    let report: Value = std::fs::read_to_string(report_path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| json!({}));
    let wallet = report
        .get("wallet_address")
        .and_then(Value::as_str)
        .unwrap_or("");
    let policy_hash = report
        .get("policy_hash")
        .and_then(Value::as_str)
        .unwrap_or("");
    let report_hash = report
        .get("report_hash")
        .and_then(Value::as_str)
        .unwrap_or("");
    let lifecycle = config
        .get("job_lifecycle")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let deliverables = config
        .get("deliverables")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let contracts = [
        "payment_token",
        "commerce_proxy",
        "router_proxy",
        "policy",
        "erc8004_registry",
    ]
    .iter()
    .filter_map(|key| {
        config.get(*key).and_then(Value::as_str).map(|address| {
            json!({
                "name": key,
                "address": address,
                "bsctrace": format!("https://bsctrace.com/address/{address}")
            })
        })
    })
    .collect::<Vec<_>>();

    Ok(json!({
        "config_path": CONFIG,
        "status": if wallet.is_empty() || policy_hash.is_empty() { "needs_report" } else { "ready" },
        "name": config.get("name").cloned().unwrap_or(json!("ERC-8183 Commerce")),
        "network": config.get("network").cloned().unwrap_or(json!("bsc-mainnet")),
        "chain_id": config.get("chain_id").cloned().unwrap_or(json!(56)),
        "agent_role": config.get("agent_role").cloned().unwrap_or(json!("provider")),
        "service_price_usd": config.get("service_price_usd").cloned().unwrap_or(json!(1.0)),
        "payment_token_symbol": config.get("payment_token_symbol").cloned().unwrap_or(json!("US")),
        "agent": {
            "wallet_address": wallet,
            "policy_hash": policy_hash,
            "report_hash": report_hash,
            "agent_endpoint": config.get("agent_endpoint").cloned().unwrap_or(json!("/skill")),
            "negotiate_endpoint": config.get("negotiate_endpoint").cloned().unwrap_or(json!("/commerce")),
            "status_endpoint": config.get("status_endpoint").cloned().unwrap_or(json!("/commerce"))
        },
        "summary": {
            "contracts": contracts.len(),
            "lifecycle_steps": lifecycle.len(),
            "deliverables": deliverables.len()
        },
        "contracts": contracts,
        "job_lifecycle": lifecycle,
        "deliverables": deliverables
    }))
}
