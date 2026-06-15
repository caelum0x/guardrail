//! Wallet controls endpoint.
//!
//! Surfaces self-custody wallet facts and configured approval caps. Read-only:
//! no allowance or signing action is performed.

use axum::Json;
use common::Address;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde_json::{json, Value};

const CONTROLS: &str = "configs/wallet/wallet_controls.json";

pub async fn wallet_controls() -> Json<Value> {
    match build() {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build() -> anyhow::Result<Value> {
    let config: Value = serde_json::from_str(&std::fs::read_to_string(CONTROLS)?)?;
    let report_path = std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| {
        config
            .get("report_path")
            .and_then(Value::as_str)
            .unwrap_or("data/run_report.json")
            .to_string()
    });
    let report: Value = serde_json::from_str(&std::fs::read_to_string(&report_path)?)?;
    let wallet = report
        .get("wallet_address")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let wallet_valid = twak_client::wallet::validate_wallet(&Address::new(wallet.clone()));
    let max_allowance = decimal_config(&config, "max_allowance_usd", Decimal::from(1500));
    let mut rows = Vec::new();
    let mut violations = 0usize;

    for spender in config
        .get("spenders")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let allowance = decimal_value(spender.get("allowance_usd")).unwrap_or(Decimal::ZERO);
        let address = spender.get("address").and_then(Value::as_str).unwrap_or("");
        let address_valid = Address::new(address).looks_valid();
        let status = if !address_valid || allowance > max_allowance {
            violations += 1;
            "violation"
        } else if allowance == Decimal::ZERO {
            "inactive"
        } else {
            "ok"
        };
        rows.push(json!({
            "name": spender.get("name").cloned().unwrap_or(json!("spender")),
            "address": address,
            "address_valid": address_valid,
            "allowance_usd": allowance.round_dp(2).to_string(),
            "status": status
        }));
    }

    let status = if !wallet_valid || violations > 0 {
        "blocking"
    } else {
        "controlled"
    };
    Ok(json!({
        "status": status,
        "config_path": CONTROLS,
        "report_path": report_path,
        "wallet": {
            "address": wallet,
            "valid": wallet_valid
        },
        "controls": {
            "expected_chain_id": config.get("expected_chain_id").cloned().unwrap_or(json!(56)),
            "approval_mode": config.get("approval_mode").cloned().unwrap_or(json!("exact_or_low_cap")),
            "max_allowance_usd": max_allowance.round_dp(2).to_string(),
            "require_quote_before_swap": config.get("require_quote_before_swap").cloned().unwrap_or(json!(true)),
            "require_twak_execution": config.get("require_twak_execution").cloned().unwrap_or(json!(true)),
            "approvals_required": twak_client::approvals::approvals_required()
        },
        "summary": {
            "spenders": rows.len(),
            "violations": violations
        },
        "spenders": rows
    }))
}

fn decimal_config(config: &Value, key: &str, default: Decimal) -> Decimal {
    decimal_value(config.get(key)).unwrap_or(default)
}

fn decimal_value(value: Option<&Value>) -> Option<Decimal> {
    value
        .and_then(Value::as_f64)
        .and_then(Decimal::from_f64)
        .or_else(|| value.and_then(Value::as_i64).map(Decimal::from))
        .or_else(|| value.and_then(Value::as_u64).map(Decimal::from))
        .or_else(|| {
            value
                .and_then(Value::as_str)
                .and_then(|raw| raw.parse::<Decimal>().ok())
        })
}
