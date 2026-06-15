//! x402 signing policy endpoint.
//!
//! Builds a deterministic example CMC x402 payment payload and signs the
//! authorization through the TWAK signing adapter. Read-only; no payment is
//! submitted and no wallet secret is loaded.

use axum::Json;
use serde_json::{json, Value};

const CONFIG: &str = "configs/x402/signing_policy.json";

pub async fn signing_policy() -> Json<Value> {
    match build() {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build() -> anyhow::Result<Value> {
    let config: Value = serde_json::from_str(&std::fs::read_to_string(CONFIG)?)?;
    let payer = std::env::var(
        config
            .get("payer_wallet_env")
            .and_then(Value::as_str)
            .unwrap_or("CMC_X402_FROM"),
    )
    .unwrap_or_else(|_| {
        config
            .get("fallback_payer_wallet")
            .and_then(Value::as_str)
            .unwrap_or("0xA9e5C0FfEe0000000000000000000000000A1b2C3")
            .to_string()
    });
    let resources = config
        .get("resources")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let first = resources.first().cloned().unwrap_or_else(|| json!({}));
    let requirements = cmc_client::x402::PaymentRequirements {
        scheme: first
            .get("scheme")
            .and_then(Value::as_str)
            .unwrap_or("exact")
            .to_string(),
        network: first
            .get("network")
            .and_then(Value::as_str)
            .unwrap_or("bsc")
            .to_string(),
        max_amount_required: first
            .get("amount_base_units")
            .and_then(Value::as_str)
            .unwrap_or("100000")
            .to_string(),
        asset: config
            .get("payment_token")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        pay_to: first
            .get("pay_to")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        resource: first
            .get("resource")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    };
    let unsigned = cmc_client::x402::PaymentPayload::from_requirements(&requirements, &payer);
    let auth = unsigned.authorization_json();
    let signed = twak_client::x402::sign_authorization(&auth, &payer);
    let payment = unsigned.with_signature(signed.signature.clone());
    let allowlist = config
        .get("primary_type_allowlist")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let denylist = config
        .get("primary_type_denylist")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    Ok(json!({
        "config_path": CONFIG,
        "name": config.get("name").cloned().unwrap_or(json!("TWAK x402 Signing Policy")),
        "status": "strict",
        "mode": config.get("mode").cloned().unwrap_or(json!("strict_default")),
        "chain_id": config.get("chain_id").cloned().unwrap_or(json!(56)),
        "headers": {
            "payment": config.get("payment_header").cloned().unwrap_or(json!("X-PAYMENT")),
            "accepts": config.get("accepts_header").cloned().unwrap_or(json!("X-PAYMENT-ACCEPTS"))
        },
        "budget": {
            "payment_token": config.get("payment_token").cloned().unwrap_or(json!("")),
            "max_per_call_base_units": config.get("max_per_call_base_units").cloned().unwrap_or(json!("0")),
            "session_budget_base_units": config.get("session_budget_base_units").cloned().unwrap_or(json!("0")),
            "validity_window_seconds": config.get("validity_window_seconds").cloned().unwrap_or(json!(600)),
            "max_future_validity_seconds": config.get("max_future_validity_seconds").cloned().unwrap_or(json!(900))
        },
        "summary": {
            "allowed_types": allowlist.len(),
            "denied_types": denylist.len(),
            "resources": resources.len(),
            "sample_signed": payment.is_signed()
        },
        "primary_type_allowlist": allowlist,
        "primary_type_denylist": denylist,
        "resources": resources,
        "sample_payment": {
            "resource": requirements.resource,
            "payer": payer,
            "authorization_hash": policy_compiler::policy_hash(auth.as_bytes()),
            "signature": signed.signature,
            "header_preview": payment.header_value()
        }
    }))
}
