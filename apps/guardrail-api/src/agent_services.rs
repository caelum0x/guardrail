//! ERC-8183 provider service catalog endpoint.
//!
//! Turns Guardrail deliverables into job offerings that can be negotiated,
//! funded, submitted, and settled through the BNB Agent SDK commerce stack.

use axum::Json;
use serde_json::{json, Value};

const CONFIG: &str = "configs/bnb/agent_services.json";

pub async fn agent_services() -> Json<Value> {
    match build() {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build() -> anyhow::Result<Value> {
    let config: Value = serde_json::from_str(&std::fs::read_to_string(CONFIG)?)?;
    let commerce_path = config
        .get("commerce_config_path")
        .and_then(Value::as_str)
        .unwrap_or("configs/bnb/erc8183_commerce.json");
    let commerce: Value = std::fs::read_to_string(commerce_path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| json!({}));
    let services = config
        .get("services")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|service| {
            let hash = policy_compiler::policy_hash(service.to_string().as_bytes());
            json!({
                "id": service.get("id").cloned().unwrap_or(json!("service")),
                "label": service.get("label").cloned().unwrap_or(json!("Service")),
                "price_usd": service.get("price_usd").cloned().unwrap_or(json!(0)),
                "sla_minutes": service.get("sla_minutes").cloned().unwrap_or(json!(0)),
                "endpoint": service.get("endpoint").cloned().unwrap_or(json!("")),
                "deliverables": service.get("deliverables").cloned().unwrap_or(json!([])),
                "required_inputs": service.get("required_inputs").cloned().unwrap_or(json!([])),
                "job_description_hash": hash
            })
        })
        .collect::<Vec<_>>();
    let total_price = services
        .iter()
        .filter_map(|service| service.get("price_usd").and_then(Value::as_f64))
        .sum::<f64>();

    Ok(json!({
        "config_path": CONFIG,
        "commerce_config_path": commerce_path,
        "name": config.get("name").cloned().unwrap_or(json!("Agent Services")),
        "provider": config.get("provider").cloned().unwrap_or(json!("Guardrail Alpha")),
        "network": config.get("network").cloned().unwrap_or(json!("bsc-mainnet")),
        "currency": config.get("currency").cloned().unwrap_or(json!("US")),
        "status": if services.is_empty() { "empty" } else { "listed" },
        "commerce": {
            "payment_token": commerce.get("payment_token").cloned().unwrap_or(json!("")),
            "commerce_proxy": commerce.get("commerce_proxy").cloned().unwrap_or(json!("")),
            "router_proxy": commerce.get("router_proxy").cloned().unwrap_or(json!("")),
            "policy": commerce.get("policy").cloned().unwrap_or(json!(""))
        },
        "summary": {
            "services": services.len(),
            "total_catalog_price_usd": (total_price * 100.0).round() / 100.0,
            "deliverable_routes": services.iter().filter_map(|service| service.get("deliverables").and_then(Value::as_array).map(Vec::len)).sum::<usize>()
        },
        "services": services
    }))
}
