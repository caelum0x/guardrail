//! CMC Agent Hub capability descriptor: `GET /cmc/capabilities`.
//!
//! Publishes the agent's CMC dataset → capability lineage (the single source of
//! truth in `configs/cmc/capabilities.json`) so a CMC Agent Hub consumer can
//! discover, verify, and call the agent's **read-only** analysis surface. The
//! descriptor names the exact `cmc-client` source for each dataset and the API
//! route / MCP tool that exposes each capability, making the integration
//! verifiable rather than asserted.
//!
//! This surface never exposes trade execution — only CMC-derived analysis.

use axum::Json;
use serde_json::{json, Value};

const CAPABILITIES_FILE: &str = "configs/cmc/capabilities.json";

pub async fn cmc_capabilities() -> Json<Value> {
    let descriptor = match std::fs::read_to_string(CAPABILITIES_FILE) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or_else(|e| {
            json!({ "error": format!("invalid {CAPABILITIES_FILE}: {e}") })
        }),
        Err(e) => json!({ "error": format!("missing {CAPABILITIES_FILE}: {e}") }),
    };

    let datasets = descriptor.get("datasets").and_then(Value::as_array).map(Vec::len).unwrap_or(0);
    let capabilities = descriptor
        .get("capabilities")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);

    Json(json!({
        "status": "ready",
        "source": CAPABILITIES_FILE,
        "summary": {
            "cmc_datasets": datasets,
            "exposed_capabilities": capabilities,
            "execution_exposed": false,
        },
        "descriptor": descriptor,
    }))
}
