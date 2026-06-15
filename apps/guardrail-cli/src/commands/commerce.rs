//! BNB / self-custody surface commands: BNB Agent SDK mapping, ERC-8183
//! commerce catalog, and the TWAK signing-policy view. These read JSON config
//! and print human-readable summaries; signing/keys always stay with TWAK.

use crate::util::json_f64;
use crate::{read_json_report, DEFAULT_AGENT_WALLET};
use common::constants::COMPETITION_CONTRACT;

pub fn run_bnb_sdk(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read BNB SDK map {config_path}"))?;
    println!("# BNB Agent SDK Map");
    println!();
    println!("config: {config_path}");
    println!(
        "source_repo: {}",
        config
            .get("source_repo")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-")
    );
    println!(
        "local_clone: {}",
        config
            .get("local_clone")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-")
    );
    println!(
        "network: {} chain_id={}",
        config
            .get("network")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("bsc-mainnet"),
        config
            .get("chain_id")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(56)
    );
    println!(
        "competition_contract: {}",
        config
            .get("competition_contract")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(COMPETITION_CONTRACT)
    );
    println!(
        "bsctrace: {}",
        config
            .get("competition_contract_bsctrace")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("https://bsctrace.com/address/0x212c61b9b72c95d95bf29cf032f5e5635629aed5")
    );
    println!();
    println!("| SDK Module | Status | Guardrail Surface |");
    println!("|:-----------|:-------|:------------------|");
    if let Some(modules) = config
        .get("sdk_modules")
        .and_then(serde_json::Value::as_array)
    {
        for module in modules {
            let name = module
                .get("module")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("module");
            let status = module
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("mapped");
            let surface = module
                .get("guardrail_surface")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-");
            println!("| {name} | {status} | {surface} |");
        }
    }
    println!();
    println!("contracts:");
    if let Some(contracts) = config
        .get("sdk_contracts")
        .and_then(serde_json::Value::as_object)
    {
        for (name, address) in contracts {
            println!("  {name}: {}", address.as_str().unwrap_or("-"));
        }
    }
    Ok(())
}

pub fn run_commerce(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read commerce config {config_path}"))?;
    let report_path = config
        .get("report_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("data/run_report.json");
    let report = read_json_report(report_path).unwrap_or_else(|| serde_json::json!({}));
    let wallet = report
        .get("wallet_address")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("-");
    let policy_hash = report
        .get("policy_hash")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("-");
    let report_hash = report
        .get("report_hash")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("-");

    println!("# ERC-8183 Commerce");
    println!();
    println!("config: {config_path}");
    println!(
        "network: {} chain_id={}",
        config
            .get("network")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("bsc-mainnet"),
        config
            .get("chain_id")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(56)
    );
    println!(
        "service_price: {:.2} {}",
        json_f64(&config, "service_price_usd").unwrap_or(0.0),
        config
            .get("payment_token_symbol")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("US")
    );
    println!("wallet: {wallet}");
    println!("policy_hash: {policy_hash}");
    println!("report_hash: {report_hash}");
    println!();
    println!("contracts:");
    for key in [
        "payment_token",
        "commerce_proxy",
        "router_proxy",
        "policy",
        "erc8004_registry",
    ] {
        println!(
            "  {key}: {}",
            config
                .get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-")
        );
    }
    println!();
    println!("| State | Guardrail Surface | Description |");
    println!("|:------|:------------------|:------------|");
    if let Some(steps) = config
        .get("job_lifecycle")
        .and_then(serde_json::Value::as_array)
    {
        for step in steps {
            let state = step
                .get("state")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-");
            let surface = step
                .get("guardrail_surface")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-");
            let description = step
                .get("description")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-");
            println!("| {state} | {surface} | {description} |");
        }
    }
    println!();
    println!("deliverables:");
    if let Some(deliverables) = config
        .get("deliverables")
        .and_then(serde_json::Value::as_array)
    {
        for deliverable in deliverables {
            if let Some(path) = deliverable.as_str() {
                println!("  {path}");
            }
        }
    }
    Ok(())
}

pub fn run_signing_policy(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read signing policy {config_path}"))?;
    let payer_env = config
        .get("payer_wallet_env")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("CMC_X402_FROM");
    let payer = std::env::var(payer_env).unwrap_or_else(|_| {
        config
            .get("fallback_payer_wallet")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(DEFAULT_AGENT_WALLET)
            .to_string()
    });
    let resources = config
        .get("resources")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let first = resources
        .first()
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let requirements = cmc_client::x402::PaymentRequirements {
        scheme: first
            .get("scheme")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("exact")
            .to_string(),
        network: first
            .get("network")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("bsc")
            .to_string(),
        max_amount_required: first
            .get("amount_base_units")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("100000")
            .to_string(),
        asset: config
            .get("payment_token")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
        pay_to: first
            .get("pay_to")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
        resource: first
            .get("resource")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
    };
    let unsigned = cmc_client::x402::PaymentPayload::from_requirements(&requirements, &payer);
    let authorization = unsigned.authorization_json();
    let signed = twak_client::x402::sign_authorization(&authorization, &payer);

    println!("# x402 Signing Policy");
    println!();
    println!("config: {config_path}");
    println!(
        "mode: {}",
        config
            .get("mode")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("strict_default")
    );
    println!(
        "payment_token: {}",
        config
            .get("payment_token")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-")
    );
    println!(
        "max_per_call_base_units: {}",
        config
            .get("max_per_call_base_units")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("0")
    );
    println!(
        "session_budget_base_units: {}",
        config
            .get("session_budget_base_units")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("0")
    );
    println!("sample_payer: {payer}");
    println!(
        "sample_authorization_hash: {}",
        policy_compiler::policy_hash(authorization.as_bytes())
    );
    println!("sample_signature: {}", signed.signature);
    println!();
    println!("allowlist:");
    if let Some(values) = config
        .get("primary_type_allowlist")
        .and_then(serde_json::Value::as_array)
    {
        for value in values {
            if let Some(value) = value.as_str() {
                println!("  {value}");
            }
        }
    }
    println!("denylist:");
    if let Some(values) = config
        .get("primary_type_denylist")
        .and_then(serde_json::Value::as_array)
    {
        for value in values {
            if let Some(value) = value.as_str() {
                println!("  {value}");
            }
        }
    }
    println!();
    println!("| Resource | Amount | Network | Pay To |");
    println!("|:---------|-------:|:--------|:-------|");
    for resource in resources {
        let label = resource
            .get("label")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("resource");
        let amount = resource
            .get("amount_base_units")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("0");
        let network = resource
            .get("network")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("bsc");
        let pay_to = resource
            .get("pay_to")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-");
        println!("| {label} | {amount} | {network} | {pay_to} |");
    }
    Ok(())
}
