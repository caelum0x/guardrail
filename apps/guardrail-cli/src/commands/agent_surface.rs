//! Agent / competition-surface display commands: scorecard, SDK catalog, agent
//! services, agent card, and the job simulator. These read JSON config/state and
//! print human-readable summaries; none mutate state.

use crate::util::{count_ext_cli, count_files_cli};
use crate::{read_json_report, DEFAULT_UNIVERSE};

pub fn run_scorecard(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read scorecard config {config_path}"))?;
    let report = read_json_report("data/run_report.json");
    let report_present = report.is_some();
    let wallet_present = report
        .as_ref()
        .and_then(|value| value.get("wallet_address"))
        .and_then(serde_json::Value::as_str)
        .map(|value| !value.is_empty())
        .unwrap_or(false);
    let policy_hash_present = report
        .as_ref()
        .and_then(|value| value.get("policy_hash"))
        .and_then(serde_json::Value::as_str)
        .map(|value| !value.is_empty())
        .unwrap_or(false);
    let daily_trade = report
        .as_ref()
        .and_then(|value| value.get("daily_trade"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let mut facts = std::collections::BTreeMap::new();
    facts.insert("report_present", report_present);
    facts.insert("wallet_present", wallet_present);
    facts.insert("policy_hash_present", policy_hash_present);
    let confirmed_txs = report
        .as_ref()
        .and_then(|value| value.get("trades"))
        .and_then(serde_json::Value::as_array)
        .map(|trades| !trades.is_empty())
        .unwrap_or(false);
    facts.insert("confirmed_txs", confirmed_txs);
    facts.insert("registered", false);
    facts.insert("daily_trade", daily_trade);
    facts.insert(
        "eligible_assets",
        std::path::Path::new(DEFAULT_UNIVERSE).exists(),
    );
    facts.insert(
        "skill_present",
        std::path::Path::new("skills/cmc-regime-routed-alpha/skill.yaml").exists(),
    );
    facts.insert("twak_only", true);
    facts.insert(
        "bnb_sdk_mapped",
        std::path::Path::new("integrations/bnbagent-sdk/bnbagent").exists(),
    );
    facts.insert(
        "commerce_ready",
        std::path::Path::new("configs/bnb/erc8183_commerce.json").exists(),
    );
    facts.insert(
        "audit_ready",
        std::path::Path::new("configs/audit/export_manifest.json").exists(),
    );

    println!("# Judge Scorecard");
    println!();
    println!("config: {config_path}");
    println!();
    println!("| Section | Facts | Weight | Status | Evidence |");
    println!("|:--------|------:|-------:|:-------|:---------|");
    if let Some(sections) = config.get("sections").and_then(serde_json::Value::as_array) {
        for section in sections {
            let label = section
                .get("label")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Section");
            let weight = section
                .get("weight")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let required = section
                .get("required_facts")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            let passed = required
                .iter()
                .filter(|fact| {
                    fact.as_str()
                        .and_then(|name| facts.get(name))
                        .copied()
                        .unwrap_or(false)
                })
                .count();
            let evidence = section
                .get("evidence_routes")
                .and_then(serde_json::Value::as_array)
                .map(|routes| {
                    routes
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let status = if passed == required.len() {
                "ready"
            } else {
                "partial"
            };
            println!(
                "| {label} | {}/{} | {:.0} | {status} | {evidence} |",
                passed,
                required.len(),
                weight
            );
        }
    }
    Ok(())
}

pub fn run_sdk_catalog() -> anyhow::Result<()> {
    let root = std::path::Path::new("integrations/bnbagent-sdk");
    let modules = [
        "erc8004", "erc8183", "x402", "signing", "wallets", "storage", "erc20", "core", "networks",
    ];
    println!("# BNB Agent SDK Catalog");
    println!();
    println!("root: {}", root.display());
    println!("files: {}", count_files_cli(root));
    println!("tests: {}", count_files_cli(&root.join("tests")));
    println!("abis: {}", count_ext_cli(root, "json"));
    println!();
    println!("| Module | Present | Files | Path |");
    println!("|:-------|:--------|------:|:-----|");
    for module in modules {
        let path = root.join("bnbagent").join(module);
        println!(
            "| {module} | {} | {} | {} |",
            path.exists(),
            count_files_cli(&path),
            path.display()
        );
    }
    println!();
    println!("examples:");
    if let Ok(entries) = std::fs::read_dir(root.join("examples")) {
        let mut rows = entries.filter_map(Result::ok).collect::<Vec<_>>();
        rows.sort_by_key(|entry| entry.file_name());
        for entry in rows {
            if entry.path().is_dir() {
                println!(
                    "  {} ({})",
                    entry.file_name().to_string_lossy(),
                    count_files_cli(&entry.path())
                );
            }
        }
    }
    Ok(())
}

pub fn run_agent_services(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read agent services config {config_path}"))?;
    println!("# ERC-8183 Provider Services");
    println!();
    println!("config: {config_path}");
    println!(
        "provider: {}",
        config
            .get("provider")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Guardrail Alpha")
    );
    println!(
        "network: {}",
        config
            .get("network")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("bsc-mainnet")
    );
    println!();
    println!("| Service | Price | SLA | Endpoint | Deliverables | Hash |");
    println!("|:--------|------:|----:|:---------|:-------------|:-----|");
    if let Some(services) = config.get("services").and_then(serde_json::Value::as_array) {
        for service in services {
            let label = service
                .get("label")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Service");
            let price = service
                .get("price_usd")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let sla = service
                .get("sla_minutes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let endpoint = service
                .get("endpoint")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-");
            let deliverables = service
                .get("deliverables")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let hash = policy_compiler::policy_hash(service.to_string().as_bytes());
            println!(
                "| {label} | {:.2} | {sla}m | {endpoint} | {deliverables} | {}... |",
                price,
                &hash[..12]
            );
        }
    }
    Ok(())
}

pub fn run_agent_card(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read agent card config {config_path}"))?;
    let card = serde_json::json!({
        "type": "https://eips.ethereum.org/EIPS/eip-8004#registration-v1",
        "name": config.get("name").cloned().unwrap_or(serde_json::json!("Guardrail Alpha")),
        "description": config.get("description").cloned().unwrap_or(serde_json::json!("")),
        "image": config.get("image").cloned().unwrap_or(serde_json::json!("")),
        "services": config.get("endpoints").cloned().unwrap_or(serde_json::json!([])),
        "registrations": [{
            "agentId": config.get("agent_id_hint").and_then(serde_json::Value::as_u64).unwrap_or(8004),
            "agentRegistry": format!(
                "eip155:{}:{}",
                config.get("chain_id").and_then(serde_json::Value::as_u64).unwrap_or(56),
                config.get("identity_registry").and_then(serde_json::Value::as_str).unwrap_or("")
            )
        }],
        "supportedTrust": config.get("supported_trust").cloned().unwrap_or(serde_json::json!([]))
    });
    let canonical = serde_json::to_string(&card)?;
    println!("# Agent Card");
    println!();
    println!("config: {config_path}");
    println!(
        "registration_hash: {}",
        policy_compiler::policy_hash(canonical.as_bytes())
    );
    println!();
    println!("| Endpoint | URL | Version | Capabilities |");
    println!("|:---------|:----|:--------|:-------------|");
    if let Some(endpoints) = card.get("services").and_then(serde_json::Value::as_array) {
        for endpoint in endpoints {
            let name = endpoint
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-");
            let url = endpoint
                .get("endpoint")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-");
            let version = endpoint
                .get("version")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-");
            let capabilities = endpoint
                .get("capabilities")
                .and_then(serde_json::Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            println!("| {name} | {url} | {version} | {capabilities} |");
        }
    }
    Ok(())
}

pub fn run_job_simulator(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read job simulator config {config_path}"))?;
    let services_path = config
        .get("agent_services_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("configs/bnb/agent_services.json");
    let services = read_json_report(services_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read agent services {services_path}"))?;
    let selected = config
        .get("selected_service_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("submission_evidence_pack");
    let service = services
        .get("services")
        .and_then(serde_json::Value::as_array)
        .and_then(|items| {
            items.iter().find(|service| {
                service
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .map(|id| id == selected)
                    .unwrap_or(false)
            })
        })
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let description = serde_json::json!({
        "version": 1,
        "task": service.get("label").cloned().unwrap_or(serde_json::json!("Service")),
        "terms": {
            "deliverables": service.get("deliverables").cloned().unwrap_or(serde_json::json!([])),
            "required_inputs": service.get("required_inputs").cloned().unwrap_or(serde_json::json!([])),
            "sla_minutes": service.get("sla_minutes").cloned().unwrap_or(serde_json::json!(0))
        },
        "price": service.get("price_usd").cloned().unwrap_or(serde_json::json!(0)),
        "currency": services.get("currency").cloned().unwrap_or(serde_json::json!("US"))
    });
    let job_hash = policy_compiler::policy_hash(description.to_string().as_bytes());
    let manifest = serde_json::json!({
        "version": 1,
        "chain_id": config.get("chain_id").cloned().unwrap_or(serde_json::json!(56)),
        "contracts": config.get("contracts").cloned().unwrap_or(serde_json::json!({})),
        "response": {
            "content": format!("Guardrail deliverable package for service {selected}"),
            "content_type": "application/json"
        },
        "metadata": {
            "service_id": selected,
            "deliverable_url": config.get("deliverable_url").cloned().unwrap_or(serde_json::json!("http://localhost:8080/audit-manifest")),
            "provider_wallet": config.get("provider_wallet").cloned().unwrap_or(serde_json::json!(""))
        }
    });
    let manifest_hash = policy_compiler::policy_hash(manifest.to_string().as_bytes());

    println!("# ERC-8183 Job Simulator");
    println!();
    println!("config: {config_path}");
    println!("service_id: {selected}");
    println!("description_hash: {job_hash}");
    println!("deliverable_hash: {manifest_hash}");
    println!();
    println!("| Step | State | Description Hash | Deliverable Hash |");
    println!("|-----:|:------|:-----------------|:-----------------|");
    if let Some(states) = config
        .get("status_sequence")
        .and_then(serde_json::Value::as_array)
    {
        for (index, state) in states.iter().enumerate() {
            let state = state.as_str().unwrap_or("-");
            let delivery = if index >= 2 {
                &manifest_hash[..12]
            } else {
                "-"
            };
            println!(
                "| {} | {state} | {}... | {delivery} |",
                index + 1,
                &job_hash[..12]
            );
        }
    }
    Ok(())
}
