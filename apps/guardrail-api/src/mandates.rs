//! Strategy mandate library endpoint.
//!
//! Compiles product-owned natural-language mandates into validated policy
//! fingerprints so the dashboard can show NL mandate -> risk policy evidence.

use axum::Json;
use serde_json::{json, Value};

const MANDATES: &str = "configs/mandates/strategy_mandates.json";

pub async fn mandates() -> Json<Value> {
    match build() {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build() -> anyhow::Result<Value> {
    let raw: Value = serde_json::from_str(&std::fs::read_to_string(MANDATES)?)?;
    let mut rows = Vec::new();
    for item in raw.as_array().cloned().unwrap_or_default() {
        let mandate = item.get("mandate").and_then(Value::as_str).unwrap_or("");
        let compiled = policy_compiler::compile_mandate(mandate)?;
        rows.push(json!({
            "id": item.get("id").cloned().unwrap_or(json!("unknown")),
            "label": item.get("label").cloned().unwrap_or(json!("Mandate")),
            "mandate": mandate,
            "policy_hash": compiled.hash,
            "policy": {
                "max_total_drawdown_pct": compiled.policy.max_total_drawdown_pct.to_string(),
                "max_daily_drawdown_pct": compiled.policy.max_daily_drawdown_pct.to_string(),
                "max_position_pct": compiled.policy.max_position_pct.to_string(),
                "max_new_position_pct": compiled.policy.max_new_position_pct.to_string(),
                "min_stable_reserve_pct": compiled.policy.min_stable_reserve_pct.to_string(),
                "max_slippage_pct": compiled.policy.max_slippage_pct.to_string(),
                "execution_layer": compiled.policy.execution_layer,
                "require_quote_before_swap": compiled.policy.require_quote_before_swap,
                "daily_trade_enabled": compiled.policy.daily_trade_requirement.enabled,
                "min_trades_per_day": compiled.policy.daily_trade_requirement.min_trades_per_day
            }
        }));
    }
    Ok(json!({
        "path": MANDATES,
        "count": rows.len(),
        "mandates": rows
    }))
}
