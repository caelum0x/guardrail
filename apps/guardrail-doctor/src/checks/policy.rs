//! Risk-policy validation and allowlist/universe consistency checks.

use market_data::Universe;
use risk_engine::RiskPolicy;

use crate::check::{CheckResult, Status};

/// Each risk policy parses via `RiskPolicy::from_json_str` AND passes
/// `policy_compiler::validate_policy`.
pub fn check_risk_policy(path: &str) -> CheckResult {
    let name = format!("risk policy: {path}");
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) => return CheckResult::fail(name, format!("read failed: {err}")),
    };
    let policy = match RiskPolicy::from_json_str(&raw) {
        Ok(policy) => policy,
        Err(err) => return CheckResult::fail(name, format!("parse failed: {err}")),
    };
    match policy_compiler::validate_policy(&policy) {
        Ok(()) => CheckResult::pass(name, "parsed and validated"),
        Err(err) => CheckResult::fail(name, format!("validation failed: {err}")),
    }
}

/// Every symbol in a policy's `allowed_assets` must be present and enabled in the
/// eligible universe. This guards the exact drift that once broke live trading —
/// the policy allowing a symbol the universe no longer ships.
pub fn check_allowlist_subset(policy_path: &str, universe_path: &str) -> CheckResult {
    let name = format!("allowlist ⊆ universe: {policy_path}");

    let universe = match Universe::load(universe_path) {
        Ok(u) => u,
        Err(err) => return CheckResult::fail(name, format!("universe load failed: {err}")),
    };
    let raw = match std::fs::read_to_string(policy_path) {
        Ok(raw) => raw,
        Err(err) => return CheckResult::fail(name, format!("read failed: {err}")),
    };
    let policy = match RiskPolicy::from_json_str(&raw) {
        Ok(policy) => policy,
        Err(err) => return CheckResult::fail(name, format!("parse failed: {err}")),
    };

    // An empty allowlist means "any eligible asset" — nothing to cross-check.
    if policy.allowed_assets.is_empty() {
        return CheckResult::warn(name, "allowlist empty (allows any eligible asset)");
    }

    let missing: Vec<&str> = policy
        .allowed_assets
        .iter()
        .filter(|sym| !universe.is_eligible(sym))
        .map(String::as_str)
        .collect();

    if missing.is_empty() {
        CheckResult::new(
            name,
            Status::Pass,
            format!("all {} allowed asset(s) are eligible", policy.allowed_assets.len()),
        )
    } else {
        CheckResult::fail(name, format!("not in universe: {}", missing.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::{RISK_POLICY_FILES, UNIVERSE_FILE};

    /// Resolve a repo-relative path independently of the test runner's cwd.
    fn repo_path(rel: &str) -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(rel)
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn allowlist_subset_holds_for_repo_policies() {
        let universe = repo_path(UNIVERSE_FILE);
        for path in RISK_POLICY_FILES {
            let result = check_allowlist_subset(&repo_path(path), &universe);
            assert_ne!(
                result.status,
                Status::Fail,
                "{path}: allowlist drifted from universe: {}",
                result.detail
            );
        }
    }
}
