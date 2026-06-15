//! Submission and reporting commands: the operator/judge briefing, hackathon
//! prize-evidence map, audit-artifact manifest, the offline Markdown run report,
//! and the DoraHacks submission summary. These read the persisted run report and
//! submission configs and render human-readable summaries.

use crate::read_json_report;
use crate::util::{json_f64, json_num_fmt, json_str};
use common::constants::COMPETITION_CONTRACT;

pub fn run_briefing(report_path: &str, config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read briefing config {config_path}"))?;
    let report = read_json_report(report_path);
    let title = config
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Submission Briefing");
    let kill_switch = report
        .as_ref()
        .and_then(|value| value.get("kill_switch"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let status = if report.is_none() || kill_switch {
        "blocking"
    } else {
        "ready"
    };

    println!("# {title}");
    println!();
    println!("status: {status}");
    println!("report: {report_path}");
    if let Some(report) = &report {
        println!(
            "run_id: {}",
            report
                .get("run_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-")
        );
        println!(
            "nav_usd: {}",
            report
                .get("nav_usd")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-")
        );
        println!(
            "wallet: {}",
            report
                .get("wallet_address")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-")
        );
    }
    println!();
    println!("claims:");
    if let Some(claims) = config.get("claims").and_then(serde_json::Value::as_array) {
        for claim in claims {
            if let Some(claim) = claim.as_str() {
                println!("  - {claim}");
            }
        }
    }
    println!();
    println!("artifacts:");
    if let Some(paths) = config
        .get("artifact_paths")
        .and_then(serde_json::Value::as_array)
    {
        for path in paths {
            if let Some(path) = path.as_str() {
                println!("  {path}");
            }
        }
    }
    println!();
    println!("demo commands:");
    if let Some(commands) = config
        .get("demo_commands")
        .and_then(serde_json::Value::as_array)
    {
        for command in commands {
            if let Some(command) = command.as_str() {
                println!("  {command}");
            }
        }
    }
    Ok(())
}

pub fn run_prizes(config_path: &str, report_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read prize map {config_path}"))?;
    let report = read_json_report(report_path);
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
    let confirmed_txs = report
        .as_ref()
        .and_then(|value| value.get("trades"))
        .and_then(serde_json::Value::as_array)
        .map(|trades| !trades.is_empty())
        .unwrap_or(false);
    let daily_trade = report
        .as_ref()
        .and_then(|value| value.get("daily_trade"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(confirmed_txs);

    println!("# Prize Map");
    println!();
    println!("config: {config_path}");
    println!("report: {report_path}");
    println!();
    println!("| Category | Status | Facts | Evidence |");
    println!("|:---------|:-------|:------|:---------|");
    if let Some(items) = config.as_array() {
        for item in items {
            let label = item
                .get("label")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Prize");
            let required = item
                .get("required_facts")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            let passed = required
                .iter()
                .filter(|fact| {
                    let Some(name) = fact.as_str() else {
                        return false;
                    };
                    match name {
                        "report_present" => report_present,
                        "wallet_present" => wallet_present,
                        "policy_hash_present" => policy_hash_present,
                        "confirmed_txs" => confirmed_txs,
                        "daily_trade" => daily_trade,
                        _ => false,
                    }
                })
                .count();
            let status = if passed == required.len() {
                "ready"
            } else {
                "partial"
            };
            let evidence = item
                .get("evidence_paths")
                .and_then(serde_json::Value::as_array)
                .map(|paths| {
                    paths
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .take(3)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            println!(
                "| {} | {} | {}/{} | {} |",
                label,
                status,
                passed,
                required.len(),
                evidence
            );
        }
    }
    Ok(())
}

pub fn run_audit_manifest(config_path: &str) -> anyhow::Result<()> {
    let manifest = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read audit manifest {config_path}"))?;
    let artifacts = manifest
        .get("artifacts")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut present = 0usize;
    let mut missing_required = 0usize;
    let mut total_bytes = 0u64;
    let mut rows = Vec::new();
    for artifact in artifacts {
        let label = artifact
            .get("label")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("artifact")
            .to_string();
        let path = artifact
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string();
        let required = artifact
            .get("required")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let bytes = std::fs::read(&path).ok();
        let exists = bytes.is_some();
        if exists {
            present += 1;
        } else if required {
            missing_required += 1;
        }
        let size = bytes.as_ref().map(|b| b.len() as u64).unwrap_or(0);
        total_bytes += size;
        let hash = bytes
            .as_ref()
            .map(|b| policy_compiler::policy_hash(b))
            .unwrap_or_else(|| "-".to_string());
        rows.push((label, path, required, exists, size, hash));
    }

    println!("# Audit Manifest");
    println!();
    println!("config: {config_path}");
    println!(
        "name: {}",
        manifest
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Submission Audit")
    );
    println!(
        "status: {}",
        if missing_required == 0 {
            "ready"
        } else {
            "missing_required"
        }
    );
    println!("artifacts: {present}/{}", rows.len());
    println!("missing_required: {missing_required}");
    println!("total_bytes: {total_bytes}");
    println!();
    println!("| Artifact | Status | Bytes | SHA-256 |");
    println!("|:---------|:-------|------:|:--------|");
    for (label, path, required, exists, size, hash) in rows {
        let status = if exists {
            "present"
        } else if required {
            "missing"
        } else {
            "optional"
        };
        let short_hash = if hash.len() > 20 {
            format!("{}...{}", &hash[..12], &hash[hash.len() - 8..])
        } else {
            hash
        };
        println!("| {label} ({path}) | {status} | {size} | {short_hash} |");
    }

    println!();
    println!("routes:");
    if let Some(routes) = manifest.get("routes").and_then(serde_json::Value::as_array) {
        for route in routes {
            if let Some(path) = route.as_str() {
                println!("  {path}");
            }
        }
    }
    Ok(())
}

/// Render an offline Markdown run report from the agent's persisted JSON state.
///
/// If the file is missing, prints a friendly note and returns `Ok(())` so the
/// command is safe to run before the agent has produced any state.
pub fn run_report(report_path: &str) -> anyhow::Result<()> {
    let raw = match std::fs::read_to_string(report_path) {
        Ok(raw) => raw,
        Err(_) => {
            println!(
                "note: run report '{report_path}' not found; run the agent first to generate it"
            );
            return Ok(());
        }
    };

    let state: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("failed to parse run report '{report_path}': {e}"))?;

    let run_id = json_str(&state, "run_id");
    let mode = json_str(&state, "mode");
    let regime = json_str(&state, "regime");

    println!("# Guardrail Run Report");
    println!();
    println!("- run_id: {run_id}");
    println!("- mode: {mode}");
    println!("- regime: {regime}");

    let starting_nav = json_f64(&state, "starting_nav_usd");
    let current_nav = json_f64(&state, "nav_usd");
    let total_return = match (starting_nav, current_nav) {
        (Some(start), Some(now)) if start != 0.0 => Some((now - start) / start * 100.0),
        _ => None,
    };

    let kill_switch = state
        .get("kill_switch")
        .and_then(|v| v.as_bool())
        .map(|b| if b { "TRIGGERED" } else { "ok" }.to_string())
        .unwrap_or_else(|| json_str(&state, "kill_switch"));

    let events = state
        .get("events")
        .and_then(|v| v.as_u64())
        .map(|n| n.to_string())
        .unwrap_or_else(|| json_str(&state, "events"));

    println!();
    println!("## Metrics");
    println!();
    println!("| Metric | Value |");
    println!("|:-------|------:|");
    println!(
        "| Starting NAV (USD) | {} |",
        starting_nav
            .map(|n| format!("{n:.2}"))
            .unwrap_or_else(|| "n/a".to_string())
    );
    println!(
        "| Current NAV (USD) | {} |",
        current_nav
            .map(|n| format!("{n:.2}"))
            .unwrap_or_else(|| "n/a".to_string())
    );
    println!(
        "| Total Return % | {} |",
        total_return
            .map(|n| format!("{n:.2}"))
            .unwrap_or_else(|| "n/a".to_string())
    );
    println!(
        "| Total Drawdown % | {} |",
        json_num_fmt(&state, "total_drawdown_pct")
    );
    println!("| Events | {events} |");
    println!("| Kill Switch | {kill_switch} |");

    println!();
    println!("## Positions");
    println!();
    println!("| Symbol | Weight % | Value (USD) |");
    println!("|:-------|---------:|------------:|");
    match state.get("positions").and_then(|v| v.as_array()) {
        Some(positions) if !positions.is_empty() => {
            for p in positions {
                let symbol = json_str(p, "symbol");
                let weight = json_num_fmt(p, "weight_pct");
                let value = json_num_fmt(p, "value_usd");
                println!("| {symbol} | {weight} | {value} |");
            }
        }
        _ => {
            println!("| _none_ | n/a | n/a |");
        }
    }

    let wallet = json_str(&state, "wallet_address");
    let policy_hash = json_str(&state, "policy_hash");

    println!();
    println!("## Commitments");
    println!();
    println!("- wallet_address: {wallet}");
    println!("- policy_hash: {policy_hash}");

    Ok(())
}

/// One-line pitch reused by the submission summary.
const SUBMISSION_PITCH: &str =
    "NL mandate -> hashed risk policy -> regime-routed alpha -> dual risk gate \
     + kill switch -> TWAK self-custody execution, fully logged and replayable.";

/// Track the agent competes in.
const SUBMISSION_TRACK: &str = "Track 1 — Autonomous Trading Agents";

/// Print a concise DoraHacks submission summary from the latest run report.
///
/// Reads the persisted `run_report.json` (run id, mode, NAV, drawdown, and any
/// agent-identity / proof fields) and prints the project pitch, track, the run's
/// proof fields, the competition contract, and a pointer to `SUBMISSION.md`. If
/// the report is missing it prints a friendly note and returns `Ok(())` so the
/// command is safe to run before the agent has produced any state.
pub fn run_submission(report_path: &str) -> anyhow::Result<()> {
    println!("# Guardrail Alpha — Submission Summary");
    println!();
    println!("pitch  : {SUBMISSION_PITCH}");
    println!("track  : {SUBMISSION_TRACK}");
    println!("prizes : Best Use of TWAK · Agent Hub (CMC) · BNB AI Agent SDK");
    println!();
    println!("competition_contract: {COMPETITION_CONTRACT}");
    println!("writeup             : SUBMISSION.md (repo root)");
    println!();

    let raw = match std::fs::read_to_string(report_path) {
        Ok(raw) => raw,
        Err(_) => {
            println!(
                "note: run report '{report_path}' not found; run the agent first \
                 (e.g. `cargo run -p guardrail-agent -- --config configs/paper.toml`)."
            );
            return Ok(());
        }
    };

    let state: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("failed to parse run report '{report_path}': {e}"))?;

    let run_id = json_str(&state, "run_id");
    let mode = json_str(&state, "mode");

    let nav = json_f64(&state, "nav_usd")
        .map(|n| format!("{n:.2}"))
        .unwrap_or_else(|| "n/a".to_string());
    let drawdown = json_num_fmt(&state, "total_drawdown_pct");

    let kill_switch = state
        .get("kill_switch")
        .and_then(|v| v.as_bool())
        .map(|b| if b { "TRIGGERED" } else { "ok" }.to_string())
        .unwrap_or_else(|| json_str(&state, "kill_switch"));

    println!("## Latest run");
    println!();
    println!("- run_id      : {run_id}");
    println!("- mode        : {mode}");
    println!("- NAV (USD)   : {nav}");
    println!("- drawdown %  : {drawdown}");
    println!("- kill switch : {kill_switch}");

    let wallet = json_str(&state, "wallet_address");
    let policy_hash = json_str(&state, "policy_hash");
    let report_hash = json_str(&state, "report_hash");

    println!();
    println!("## Proof");
    println!();
    println!("- agent wallet : {wallet}");
    println!("- policy_hash  : {policy_hash}");
    println!("- report_hash  : {report_hash}");
    println!("- contract     : {COMPETITION_CONTRACT}");
    println!();
    println!("See SUBMISSION.md for the full writeup, strategy, and how-to-run.");

    Ok(())
}
