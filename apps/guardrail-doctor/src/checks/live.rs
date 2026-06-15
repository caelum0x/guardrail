//! Live-mode preflight checks (run only with `--live`).
//!
//! These gate a real money go-live: required credentials are present, the
//! production config is actually in live mode, and the production policy keeps
//! conservative caps. They are env + file checks only (no network) — the
//! network reachability of the RPC and TWAK is verified by `scripts/go_live.sh`
//! (which runs the on-chain verifier against `BSC_RPC_URL`).

use risk_engine::RiskPolicy;

use crate::check::CheckResult;

const PRODUCTION_CONFIG: &str = "configs/production.toml";
const PRODUCTION_POLICY: &str = "configs/risk_policy.production.json";

/// An environment variable must be present and non-empty.
fn env_present(var: &str) -> CheckResult {
    let name = format!("live env: {var}");
    match std::env::var(var) {
        Ok(v) if !v.trim().is_empty() => CheckResult::pass(name, "set"),
        _ => CheckResult::fail(name, "missing or empty — required for live trading"),
    }
}

/// At least one of a set of variables must be present (e.g. a TWAK transport URL).
fn any_env_present(label: &str, vars: &[&str]) -> CheckResult {
    let name = format!("live env: {label}");
    let found: Vec<&str> = vars
        .iter()
        .copied()
        .filter(|v| std::env::var(v).map(|s| !s.trim().is_empty()).unwrap_or(false))
        .collect();
    if found.is_empty() {
        CheckResult::fail(name, format!("none of {vars:?} set — TWAK live transport required"))
    } else {
        CheckResult::pass(name, format!("{} configured", found.join(", ")))
    }
}

/// The production config must declare live mode.
fn check_live_mode() -> CheckResult {
    let name = "live config: production mode".to_string();
    let raw = match std::fs::read_to_string(PRODUCTION_CONFIG) {
        Ok(raw) => raw,
        Err(err) => return CheckResult::fail(name, format!("read {PRODUCTION_CONFIG} failed: {err}")),
    };
    // Lightweight scan: the [app] mode line. Avoids a full config dependency.
    let is_live = raw
        .lines()
        .any(|l| l.trim_start().starts_with("mode") && l.contains("\"live\""));
    if is_live {
        CheckResult::pass(name, "mode = \"live\"")
    } else {
        CheckResult::fail(name, format!("{PRODUCTION_CONFIG} is not mode = \"live\""))
    }
}

/// The production policy must keep conservative safety caps so a live run cannot
/// fire with reckless limits.
fn check_conservative_caps() -> CheckResult {
    let name = "live policy: conservative caps".to_string();
    let raw = match std::fs::read_to_string(PRODUCTION_POLICY) {
        Ok(raw) => raw,
        Err(err) => return CheckResult::fail(name, format!("read failed: {err}")),
    };
    let policy = match RiskPolicy::from_json_str(&raw) {
        Ok(p) => p,
        Err(err) => return CheckResult::fail(name, format!("parse failed: {err}")),
    };
    use rust_decimal::Decimal;
    let mut issues: Vec<String> = Vec::new();
    if policy.kill_switch_drawdown_pct > Decimal::from(50) {
        issues.push(format!("kill_switch_drawdown_pct {} > 50", policy.kill_switch_drawdown_pct));
    }
    if policy.max_total_drawdown_pct > Decimal::from(40) {
        issues.push(format!("max_total_drawdown_pct {} > 40", policy.max_total_drawdown_pct));
    }
    if policy.min_stable_reserve_pct < Decimal::from(5) {
        issues.push(format!("min_stable_reserve_pct {} < 5", policy.min_stable_reserve_pct));
    }
    if issues.is_empty() {
        CheckResult::pass(
            name,
            format!(
                "kill_switch {}%, max_drawdown {}%, stable_reserve {}%",
                policy.kill_switch_drawdown_pct,
                policy.max_total_drawdown_pct,
                policy.min_stable_reserve_pct
            ),
        )
    } else {
        CheckResult::fail(name, format!("reckless caps: {}", issues.join("; ")))
    }
}

/// Collect all live preflight checks.
pub fn live_checks() -> Vec<CheckResult> {
    vec![
        env_present("CMC_API_KEY"),
        env_present("BSC_RPC_URL"),
        any_env_present("TWAK transport", &["TWAK_REST_URL", "TWAK_MCP_URL"]),
        check_live_mode(),
        check_conservative_caps(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::Status;

    #[test]
    fn missing_env_fails() {
        // A name we are confident is unset.
        let r = env_present("GUARDRAIL_DEFINITELY_UNSET_VAR_XYZ");
        assert_eq!(r.status, Status::Fail);
    }

    #[test]
    fn repo_production_policy_is_conservative() {
        // The committed production policy must pass the caps gate, resolving the
        // path from the crate manifest dir so cwd does not matter.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();
        let r = check_conservative_caps();
        std::env::set_current_dir(prev).unwrap();
        assert_ne!(r.status, Status::Fail, "{}", r.detail);
    }
}
