//! ERC-8004-style agent card endpoint.
//!
//! Builds the agent discovery document from product config using the same field
//! shape as the vendored BNB Agent SDK's `AgentURIGenerator`.

use axum::Json;
use serde_json::{json, Value};

const CONFIG: &str = "configs/bnb/agent_card.json";

pub async fn agent_card() -> Json<Value> {
    match build() {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

pub async fn well_known_agent_card() -> Json<Value> {
    agent_card().await
}

fn build() -> anyhow::Result<Value> {
    let config: Value = serde_json::from_str(&std::fs::read_to_string(CONFIG)?)?;
    let endpoints = config
        .get("endpoints")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let chain_id = config.get("chain_id").and_then(Value::as_u64).unwrap_or(56);
    let registry = config
        .get("identity_registry")
        .and_then(Value::as_str)
        .unwrap_or("");
    let agent_id = config
        .get("agent_id_hint")
        .and_then(Value::as_u64)
        .unwrap_or(8004);
    let card = json!({
        "type": "https://eips.ethereum.org/EIPS/eip-8004#registration-v1",
        "name": config.get("name").cloned().unwrap_or(json!("Guardrail Alpha")),
        "description": config.get("description").cloned().unwrap_or(json!("")),
        "image": config.get("image").cloned().unwrap_or(json!("")),
        "services": endpoints,
        "registrations": [{
            "agentId": agent_id,
            "agentRegistry": format!("eip155:{chain_id}:{registry}")
        }],
        "supportedTrust": config.get("supported_trust").cloned().unwrap_or(json!([]))
    });
    let canonical = serde_json::to_string(&card)?;
    Ok(json!({
        "config_path": CONFIG,
        "status": "ready",
        "summary": {
            "services": card.get("services").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
            "registrations": card.get("registrations").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
            "supported_trust": card.get("supportedTrust").and_then(Value::as_array).map(Vec::len).unwrap_or(0)
        },
        "agent_uri": format!("data:application/json;base64,{}", base64_encode(canonical.as_bytes())),
        "registration_hash": policy_compiler::policy_hash(canonical.as_bytes()),
        "card": card
    }))
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let b0 = bytes[i];
        let b1 = bytes.get(i + 1).copied().unwrap_or(0);
        let b2 = bytes.get(i + 2).copied().unwrap_or(0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if i + 1 < bytes.len() {
            out.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < bytes.len() {
            out.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}
