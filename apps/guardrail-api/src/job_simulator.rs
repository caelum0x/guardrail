//! Local ERC-8183 job lifecycle simulator.
//!
//! Builds a deterministic job description and deliverable manifest from the
//! Guardrail service catalog. It previews commerce flow without touching chain.

use axum::Json;
use serde_json::{json, Value};

const CONFIG: &str = "configs/bnb/job_simulator.json";

pub async fn job_simulator() -> Json<Value> {
    match build() {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build() -> anyhow::Result<Value> {
    let config: Value = serde_json::from_str(&std::fs::read_to_string(CONFIG)?)?;
    let services_path = config
        .get("agent_services_path")
        .and_then(Value::as_str)
        .unwrap_or("configs/bnb/agent_services.json");
    let services_config: Value = serde_json::from_str(&std::fs::read_to_string(services_path)?)?;
    let selected = config
        .get("selected_service_id")
        .and_then(Value::as_str)
        .unwrap_or("submission_evidence_pack");
    let service = services_config
        .get("services")
        .and_then(Value::as_array)
        .and_then(|items| {
            items.iter().find(|service| {
                service
                    .get("id")
                    .and_then(Value::as_str)
                    .map(|id| id == selected)
                    .unwrap_or(false)
            })
        })
        .cloned()
        .unwrap_or_else(|| json!({}));
    let chain_id = config.get("chain_id").and_then(Value::as_u64).unwrap_or(56);
    let description = json!({
        "version": 1,
        "task": service.get("label").cloned().unwrap_or(json!("Submission Evidence Pack")),
        "terms": {
            "deliverables": service.get("deliverables").cloned().unwrap_or(json!([])),
            "required_inputs": service.get("required_inputs").cloned().unwrap_or(json!([])),
            "sla_minutes": service.get("sla_minutes").cloned().unwrap_or(json!(0))
        },
        "price": service.get("price_usd").cloned().unwrap_or(json!(0)),
        "currency": services_config.get("currency").cloned().unwrap_or(json!("US"))
    });
    let job_hash = policy_compiler::policy_hash(description.to_string().as_bytes());
    let job_id = u64::from_str_radix(&job_hash[..12], 16).unwrap_or(0);
    let manifest = json!({
        "version": 1,
        "job_id": job_id,
        "chain_id": chain_id,
        "contracts": config.get("contracts").cloned().unwrap_or(json!({})),
        "response": {
            "content": format!("Guardrail deliverable package for service {selected}"),
            "content_type": "application/json"
        },
        "metadata": {
            "service_id": selected,
            "deliverable_url": config.get("deliverable_url").cloned().unwrap_or(json!("http://localhost:8080/audit-manifest")),
            "provider_wallet": config.get("provider_wallet").cloned().unwrap_or(json!(""))
        }
    });
    let manifest_hash = policy_compiler::policy_hash(manifest.to_string().as_bytes());
    let states = config
        .get("status_sequence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(index, state)| {
            json!({
                "step": index + 1,
                "state": state,
                "description_hash": job_hash,
                "manifest_hash": if index >= 2 { manifest_hash.clone() } else { String::new() }
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "config_path": CONFIG,
        "status": "simulated",
        "service": service,
        "job": {
            "job_id": job_id,
            "description_hash": job_hash,
            "description": description
        },
        "deliverable_manifest": manifest,
        "deliverable_hash": manifest_hash,
        "lifecycle": states
    }))
}
