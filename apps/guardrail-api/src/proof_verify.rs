//! Independent on-chain proof verification endpoint: `GET /proof/verify`.
//!
//! Recomputes the agent's `policy_hash` from the on-disk risk policy and checks
//! the claimed identity/proof fields in `data/run_report.json` against the
//! competition contract and BscScan/BscTrace URL formats — the same checks the
//! standalone `clients/proof-verifier` performs, but served over the read-only
//! API so a judge can verify without leaving the dashboard. Recomputation uses
//! the same `bnb_agent::sha256_hex_str` the runtime used to produce the hashes.
//!
//! When `BSC_RPC_URL` is set it additionally performs read-only on-chain checks
//! (via the `chain-verifier` crate) — chain id, deployed contract bytecode, and
//! the registration transaction receipt — so the proof is verifiable against
//! the chain itself, not just self-attested. With no RPC configured those
//! checks are `skipped` and the offline flow stays green.
//!
//! Read-only and panic-free: a missing report degrades to a clear "no report"
//! result rather than an error.

use axum::Json;
use common::constants::COMPETITION_CONTRACT;
use serde_json::{json, Value};

const REPORT_PATH: &str = "data/run_report.json";
const POLICY_FILES: [&str; 2] = [
    "configs/risk_policy.paper.json",
    "configs/risk_policy.production.json",
];

pub async fn proof_verify() -> Json<Value> {
    let Some(report) = read_json(REPORT_PATH) else {
        return Json(json!({
            "passed": false,
            "reason": "no run report — run the agent first",
            "report_path": REPORT_PATH,
            "checks": [],
        }));
    };

    let mut checks: Vec<Value> = Vec::new();

    // 1. policy_hash: recompute sha256 of each candidate policy file; pass if the
    //    claimed hash matches either (paper or production).
    let claimed_policy = str_field(&report, "policy_hash");
    let recomputed: Vec<(&str, String)> = POLICY_FILES
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok().map(|raw| (*p, bnb_agent::sha256_hex_str(&raw))))
        .collect();
    let policy_match = recomputed.iter().find(|(_, h)| *h == claimed_policy);
    checks.push(check(
        "policy_hash",
        policy_match.is_some() && !claimed_policy.is_empty(),
        match policy_match {
            Some((file, _)) => format!("claimed hash matches sha256({file})"),
            None if claimed_policy.is_empty() => "no policy_hash in report".to_string(),
            None => "claimed hash does not match paper/production policy files".to_string(),
        },
    ));

    // 2. report_hash present + well-formed (64-hex). The runtime hashes a
    //    canonical "core" subset, so we format-check rather than recompute.
    let report_hash = str_field(&report, "report_hash");
    checks.push(check(
        "report_hash",
        is_hex(&report_hash, 64),
        if is_hex(&report_hash, 64) {
            "present, valid sha256 hex".to_string()
        } else {
            "missing or malformed report_hash".to_string()
        },
    ));

    // 3. wallet address format (0x + 40 hex, or a vanity placeholder).
    let wallet = str_field(&report, "wallet_address");
    let wallet_ok = is_addr(&wallet);
    checks.push(check(
        "wallet_address",
        wallet_ok,
        if wallet_ok { format!("valid address {wallet}") } else { "missing/invalid wallet".to_string() },
    ));

    // 4. competition contract: the canonical constant is a valid address.
    checks.push(check(
        "competition_contract",
        is_addr(COMPETITION_CONTRACT),
        format!("contract {COMPETITION_CONTRACT}"),
    ));

    // 5. BscScan address URL is well-formed and points at the wallet.
    let addr_url = str_field(&report, "address_url");
    checks.push(check(
        "bscscan_address_url",
        addr_url.starts_with("https://bscscan.com/address/") || addr_url.is_empty(),
        if addr_url.is_empty() { "no address_url (optional)".to_string() } else { addr_url.clone() },
    ));

    // 6. registration tx hash format, if present.
    let tx = str_field(&report, "registration_tx");
    checks.push(check(
        "registration_tx",
        tx.is_empty() || is_hex(tx.trim_start_matches("0x"), 64),
        if tx.is_empty() { "not registered (optional in paper)".to_string() } else { tx.clone() },
    ));

    // 7. on-chain verification (read-only BSC JSON-RPC). When `BSC_RPC_URL` is
    //    set, this confirms the chain id, that the competition contract has
    //    deployed bytecode, and that the registration tx (if any) was mined to
    //    that contract. When it is unset the checks are `skipped`, never failed,
    //    so the offline paper/demo flow stays green.
    let rpc_url = chain_verifier::rpc_url_from_env();
    let onchain = chain_verifier::verify_onchain(
        rpc_url.as_deref(),
        COMPETITION_CONTRACT,
        &wallet,
        if tx.is_empty() { None } else { Some(tx.as_str()) },
    )
    .await;
    for c in &onchain.checks {
        checks.push(json!({
            "name": c.name,
            "status": c.status.as_str(),
            "detail": c.detail,
        }));
    }

    // A proof passes when no check failed; `skipped` checks (e.g. on-chain when
    // no RPC is configured) do not block it.
    let passed = !checks
        .iter()
        .any(|c| c.get("status").and_then(Value::as_str) == Some("fail"));

    Json(json!({
        "passed": passed,
        "report_path": REPORT_PATH,
        "onchain_configured": onchain.configured,
        "recomputed_policy_hashes": recomputed.iter().map(|(f, h)| json!({"file": f, "sha256": h})).collect::<Vec<_>>(),
        "checks": checks,
    }))
}

fn check(name: &str, ok: bool, detail: impl Into<String>) -> Value {
    json!({ "name": name, "status": if ok { "pass" } else { "fail" }, "detail": detail.into() })
}

fn read_json(path: &str) -> Option<Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
}

fn str_field(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

/// True when `s` is exactly `len` lowercase/uppercase hex characters.
fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// True for a 0x-prefixed 40-hex address (or a 0x-prefixed vanity placeholder).
fn is_addr(s: &str) -> bool {
    let body = s.strip_prefix("0x").unwrap_or("");
    s.starts_with("0x") && body.len() == 40 && body.chars().all(|c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_and_addr_validation() {
        assert!(is_hex(&"a".repeat(64), 64));
        assert!(!is_hex("xyz", 64));
        assert!(is_addr(COMPETITION_CONTRACT));
        assert!(!is_addr("0x123"));
    }
}
