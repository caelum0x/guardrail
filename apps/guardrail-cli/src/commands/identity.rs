//! Self-custody and operations commands: competition registration through TWAK,
//! the agent's BNB identity / proof commitments, wallet + spender controls, exit
//! triggers against live positions, and the daily-trade heartbeat. Signing and
//! keys always stay with TWAK; these commands only inspect and prepare.

use crate::util::{decimal_from_str, decimal_value, json_decimal_or};
use crate::{
    build_warmed_snapshot, read_json_report, DEFAULT_AGENT_WALLET, DEFAULT_UNIVERSE,
    SNAPSHOT_WARMUP_STEPS,
};
use common::constants::COMPETITION_CONTRACT;
use common::Settings;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

/// Parse a TWAK transport string into a [`TwakTransport`].
///
/// Anything other than the four known surfaces falls back to the offline mock,
/// with a note, so paper mode keeps working on a typo.
fn parse_transport(transport: &str) -> twak_client::TwakTransport {
    match transport.to_ascii_lowercase().as_str() {
        "mock" => twak_client::TwakTransport::Mock,
        "rest" => twak_client::TwakTransport::Rest,
        "mcp" => twak_client::TwakTransport::Mcp,
        "cli" => twak_client::TwakTransport::Cli,
        other => {
            println!("note: unknown transport '{other}'; falling back to offline mock");
            twak_client::TwakTransport::Mock
        }
    }
}

/// Register the agent for the competition through TWAK (self-custody).
///
/// Builds a tokio runtime (keeping `main` synchronous), resolves the executor
/// for the chosen transport, fetches the wallet address, and submits the
/// registration. On any failure it prints a friendly message pointing at the
/// `twak compete register` manual fallback. The default `mock` transport keeps
/// this fully offline and deterministic.
pub fn run_register(
    transport: &str,
    base_url: Option<&str>,
    autonomous: bool,
) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    let twak_transport = parse_transport(transport);

    // Prefer an explicit flag, then the environment, before any default.
    let env_base_url = std::env::var("TWAK_BASE_URL").ok();
    let resolved_base_url = base_url.or(env_base_url.as_deref());

    let result: Result<(common::Address, twak_client::TxReceipt), twak_client::TwakError> = runtime
        .block_on(async {
            let executor =
                twak_client::executor_from(twak_transport, resolved_base_url, autonomous);
            let wallet = executor.wallet_address().await?;
            let receipt = executor.register_competition().await?;
            Ok((wallet, receipt))
        });

    match result {
        Ok((wallet, receipt)) => {
            println!("transport: {transport}");
            println!("wallet_address: {}", wallet.0);
            println!("competition_contract: {COMPETITION_CONTRACT}");
            println!("tx_hash: {}", receipt.tx_hash);
            println!("bscscan: https://bscscan.com/tx/{}", receipt.tx_hash);
        }
        Err(e) => {
            println!("registration via TWAK failed: {e}");
            println!("fallback: run `twak compete register` (self-custody) to register manually");
            println!("competition_contract: {COMPETITION_CONTRACT}");
        }
    }

    Ok(())
}

/// Print the agent's BNB identity and proof commitments as pretty JSON.
pub fn run_identity(config: &str) -> anyhow::Result<()> {
    let settings = Settings::load(config)?;
    let policy_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let policy_hash = bnb_agent::sha256_hex_str(&policy_raw);

    let wallet = std::env::var("AGENT_WALLET").unwrap_or_else(|_| DEFAULT_AGENT_WALLET.to_string());

    let identity = bnb_agent::AgentIdentity::new(settings.app.name.clone(), wallet.clone());
    let agent_id = identity.agent_id();

    let metadata = bnb_agent::AgentMetadata {
        name: settings.app.name.clone(),
        description: "Regime-routed, risk-guarded alpha agent on BSC".to_string(),
        strategy_hash: bnb_agent::sha256_hex_str("regime-routed-bsc-alpha"),
        policy_hash: policy_hash.clone(),
        version: "0.1.0".to_string(),
    };

    let erc8004 = bnb_agent::Erc8004Record::build(&identity, &metadata);
    let report_hash = bnb_agent::sha256_hex_str("pending");
    let proof = bnb_agent::AgentProof::new(
        agent_id.clone(),
        wallet.clone(),
        policy_hash.clone(),
        report_hash,
    );

    let output = serde_json::json!({
        "agent_id": agent_id,
        "wallet": wallet,
        "address_url": proof.address_url(),
        "policy_hash": policy_hash,
        "metadata": metadata,
        "erc8004": erc8004,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

pub fn run_wallet_controls(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read wallet controls {config_path}"))?;
    let report_path = config
        .get("report_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("data/run_report.json");
    let report = read_json_report(report_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read report {report_path}"))?;
    let wallet = report
        .get("wallet_address")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let wallet_valid = common::Address::new(wallet).looks_valid();
    let max_allowance = json_decimal_or(&config, "max_allowance_usd", Decimal::from(1500));

    println!("# Wallet Controls");
    println!();
    println!("config: {config_path}");
    println!("report: {report_path}");
    println!("wallet: {wallet}");
    println!("wallet_valid: {wallet_valid}");
    println!(
        "approval_mode: {}",
        config
            .get("approval_mode")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("exact_or_low_cap")
    );
    println!("max_allowance_usd: {:.2}", max_allowance);
    println!();
    println!("| Spender | Status | Allowance USD | Address |");
    println!("|:--------|:-------|--------------:|:--------|");
    if let Some(spenders) = config.get("spenders").and_then(serde_json::Value::as_array) {
        for spender in spenders {
            let name = spender
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("spender");
            let address = spender
                .get("address")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let allowance = spender
                .get("allowance_usd")
                .and_then(|value| {
                    value
                        .as_f64()
                        .and_then(Decimal::from_f64)
                        .or_else(|| value.as_i64().map(Decimal::from))
                        .or_else(|| value.as_u64().map(Decimal::from))
                        .or_else(|| value.as_str().and_then(decimal_from_str))
                })
                .unwrap_or(Decimal::ZERO);
            let status =
                if !common::Address::new(address).looks_valid() || allowance > max_allowance {
                    "violation"
                } else if allowance == Decimal::ZERO {
                    "inactive"
                } else {
                    "ok"
                };
            println!("| {name} | {status} | {:.2} | {address} |", allowance);
        }
    }
    Ok(())
}

pub fn run_exit_triggers(policy_path: &str) -> anyhow::Result<()> {
    let policy = read_json_report(policy_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read exit policy {policy_path}"))?;
    let report_path = policy
        .get("report_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("data/run_report.json");
    let universe_path = policy
        .get("universe_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(DEFAULT_UNIVERSE);
    let report = read_json_report(report_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read report {report_path}"))?;
    let universe = market_data::Universe::load(universe_path)?;
    let assets = universe.enabled_assets();
    let snapshot = build_warmed_snapshot(&assets, SNAPSHOT_WARMUP_STEPS);

    let stop_loss = json_decimal_or(&policy, "stop_loss_pct", Decimal::from(12));
    let take_profit = json_decimal_or(&policy, "take_profit_pct", Decimal::from(25));
    let warning_loss = json_decimal_or(&policy, "warning_loss_pct", Decimal::from(8));
    let warning_gain = json_decimal_or(&policy, "warning_gain_pct", Decimal::from(18));
    let ret_exit = json_decimal_or(&policy, "ret_24h_exit_pct", Decimal::from(-12));

    let mut rows = Vec::new();
    let mut exit_count = 0usize;
    let mut watch_count = 0usize;
    for position in report
        .get("positions")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let symbol = position
            .get("symbol")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string();
        let weight = decimal_value(position.get("weight_pct")).unwrap_or(Decimal::ZERO);
        let value = decimal_value(position.get("value_usd")).unwrap_or(Decimal::ZERO);
        let ret_24h = snapshot
            .get(&symbol)
            .and_then(|asset| asset.ret_24h)
            .unwrap_or(Decimal::ZERO);
        let status = if ret_24h <= -stop_loss || ret_24h <= ret_exit {
            exit_count += 1;
            "exit"
        } else if ret_24h >= take_profit {
            exit_count += 1;
            "take_profit"
        } else if ret_24h <= -warning_loss || ret_24h >= warning_gain {
            watch_count += 1;
            "watch"
        } else {
            "hold"
        };
        rows.push((symbol, status, weight, value, ret_24h));
    }
    rows.sort_by(|a, b| a.4.cmp(&b.4));

    println!("# Exit Triggers");
    println!();
    println!("policy: {policy_path}");
    println!("report: {report_path}");
    println!("universe: {universe_path}");
    println!(
        "status: {}",
        if exit_count > 0 {
            "exit"
        } else if watch_count > 0 {
            "watch"
        } else {
            "hold"
        }
    );
    println!("thresholds: stop_loss={}%, take_profit={}%, warning_loss={}%, warning_gain={}%, ret_24h_exit={}%", stop_loss, take_profit, warning_loss, warning_gain, ret_exit);
    println!();
    println!("| Symbol | Status | Weight % | Value USD | 24h % |");
    println!("|:-------|:-------|---------:|----------:|------:|");
    for (symbol, status, weight, value, ret_24h) in rows {
        println!(
            "| {:<6} | {:<11} | {:>8.2} | {:>9.2} | {:>5.2} |",
            symbol, status, weight, value, ret_24h
        );
    }
    Ok(())
}

pub fn run_heartbeat(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read heartbeat config {config_path}"))?;
    let report_path = config
        .get("report_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("data/run_report.json");
    let report = read_json_report(report_path).unwrap_or_else(|| serde_json::json!({}));
    let nav = report
        .get("nav_usd")
        .and_then(serde_json::Value::as_str)
        .and_then(decimal_from_str)
        .unwrap_or_else(|| Decimal::from(10_000));
    let max_pct = json_decimal_or(&config, "max_heartbeat_trade_pct", Decimal::from(2));
    let min_notional = json_decimal_or(&config, "min_notional_usd", Decimal::from(25));
    let target = json_decimal_or(&config, "target_notional_usd", Decimal::from(100));
    let max_notional = json_decimal_or(&config, "max_notional_usd", Decimal::from(200));
    let planned = target
        .min(max_notional)
        .min(nav * max_pct / Decimal::from(100))
        .max(min_notional);
    let daily_trade = report
        .get("daily_trade")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let from = config
        .get("preferred_pair")
        .and_then(|pair| pair.get("from"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("USDT");
    let to = config
        .get("preferred_pair")
        .and_then(|pair| pair.get("to"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("WBNB");

    println!("# Daily Trade Heartbeat");
    println!();
    println!("config: {config_path}");
    println!("report: {report_path}");
    println!(
        "status: {}",
        if daily_trade {
            "satisfied"
        } else {
            "due_or_unverified"
        }
    );
    println!(
        "requirement: {} trade/day, max heartbeat {}% NAV",
        config
            .get("min_trades_per_day")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1),
        max_pct
    );
    println!("nav_usd: {:.2}", nav);
    println!("planned_pair: {from} -> {to}");
    println!("planned_notional_usd: {:.2}", planned);
    println!(
        "execution_path: {}",
        config
            .get("execution_path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("risk_gate -> TWAK quote -> final risk -> TWAK swap")
    );
    println!();
    println!(
        "quote_command: cargo run -p guardrail-cli -- quote --from {from} --to {to} --amount {:.2}",
        planned
    );
    Ok(())
}
