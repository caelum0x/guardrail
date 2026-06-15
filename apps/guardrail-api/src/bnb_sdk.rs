//! BNB Agent SDK integration map endpoint.
//!
//! Read-only evidence surface that maps the cloned Python SDK modules to the
//! Rust runtime, TWAK adapter, dashboard, and submission artifacts.

use axum::Json;
use serde_json::{json, Value};

const CONFIG: &str = "configs/bnb/bnb_agent_sdk_map.json";

pub async fn bnb_sdk() -> Json<Value> {
    match build() {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build() -> anyhow::Result<Value> {
    let config: Value = serde_json::from_str(&std::fs::read_to_string(CONFIG)?)?;
    let local_clone = config
        .get("local_clone")
        .and_then(Value::as_str)
        .unwrap_or("integrations/bnbagent-sdk");
    let modules = config
        .get("sdk_modules")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let implemented = modules
        .iter()
        .filter(|module| {
            module
                .get("status")
                .and_then(Value::as_str)
                .map(|status| status != "mapped")
                .unwrap_or(false)
        })
        .count();
    Ok(json!({
        "config_path": CONFIG,
        "source_repo": config.get("source_repo").cloned().unwrap_or(json!("")),
        "local_clone": local_clone,
        "network": config.get("network").cloned().unwrap_or(json!("bsc-mainnet")),
        "chain_id": config.get("chain_id").cloned().unwrap_or(json!(56)),
        "competition_contract": config.get("competition_contract").cloned().unwrap_or(json!("")),
        "competition_contract_bsctrace": config.get("competition_contract_bsctrace").cloned().unwrap_or(json!("")),
        "summary": {
            "modules": modules.len(),
            "implemented_or_referenced": implemented,
            "contracts": config.get("sdk_contracts").and_then(Value::as_object).map(|v| v.len()).unwrap_or(0),
            "local_files": count_files(std::path::Path::new(local_clone)),
            "local_modules_present": local_modules_present(local_clone)
        },
        "sdk_modules": modules,
        "sdk_contracts": config.get("sdk_contracts").cloned().unwrap_or(json!({}))
    }))
}

fn count_files(path: &std::path::Path) -> usize {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                count_files(&path)
            } else {
                1
            }
        })
        .sum()
}

fn local_modules_present(root: &str) -> usize {
    [
        "bnbagent/erc8004",
        "bnbagent/erc8183",
        "bnbagent/x402",
        "bnbagent/signing",
        "bnbagent/wallets",
        "bnbagent/storage",
        "bnbagent/erc20",
        "bnbagent/core",
        "bnbagent/networks",
    ]
    .iter()
    .filter(|path| std::path::Path::new(root).join(path).exists())
    .count()
}
