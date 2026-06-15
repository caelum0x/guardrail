//! guardrail-doctor: preflight / readiness checks for the Guardrail stack.
//!
//! Runs a series of named checks (config loading, risk-policy validation,
//! eligible-asset universe, data-directory writability), prints a checklist,
//! and exits with code 1 if any check fails.

use common::Settings;
use market_data::Universe;
use risk_engine::RiskPolicy;

/// Outcome severity for a single check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Pass,
    Warn,
    Fail,
}

impl Status {
    fn mark(self) -> &'static str {
        match self {
            Status::Pass => "\u{2713}", // checkmark
            Status::Warn => "!",
            Status::Fail => "\u{2717}", // cross
        }
    }
}

/// The result of running a single named check.
struct CheckResult {
    name: String,
    status: Status,
    detail: String,
}

impl CheckResult {
    fn new(name: impl Into<String>, status: Status, detail: impl Into<String>) -> Self {
        CheckResult {
            name: name.into(),
            status,
            detail: detail.into(),
        }
    }
}

const CONFIG_FILES: [&str; 3] = [
    "configs/paper.toml",
    "configs/production.toml",
    "configs/backtest.toml",
];

const RISK_POLICY_FILES: [&str; 2] = [
    "configs/risk_policy.paper.json",
    "configs/risk_policy.production.json",
];

const UNIVERSE_FILE: &str = "configs/eligible_assets.bsc.json";
const DATA_DIR: &str = "data";

/// Check 1: each config loads via `Settings::load`. Missing files are a warn,
/// not a fail.
fn check_config(path: &str) -> CheckResult {
    let name = format!("config: {path}");
    if !std::path::Path::new(path).exists() {
        return CheckResult::new(name, Status::Warn, "file absent (skipped)");
    }
    match Settings::load(path) {
        Ok(settings) => CheckResult::new(
            name,
            Status::Pass,
            format!("loaded; risk.policy_path = {}", settings.risk.policy_path),
        ),
        Err(err) => CheckResult::new(name, Status::Fail, format!("load failed: {err}")),
    }
}

/// Check 2: each risk policy parses via `RiskPolicy::from_json_str` AND passes
/// `policy_compiler::validate_policy`.
fn check_risk_policy(path: &str) -> CheckResult {
    let name = format!("risk policy: {path}");
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) => return CheckResult::new(name, Status::Fail, format!("read failed: {err}")),
    };
    let policy = match RiskPolicy::from_json_str(&raw) {
        Ok(policy) => policy,
        Err(err) => return CheckResult::new(name, Status::Fail, format!("parse failed: {err}")),
    };
    match policy_compiler::validate_policy(&policy) {
        Ok(()) => CheckResult::new(name, Status::Pass, "parsed and validated"),
        Err(err) => CheckResult::new(name, Status::Fail, format!("validation failed: {err}")),
    }
}

/// Check 3: the eligible-asset universe loads and has at least one enabled asset.
fn check_universe(path: &str) -> CheckResult {
    let name = format!("universe: {path}");
    match Universe::load(path) {
        Ok(universe) => {
            let count = universe.enabled_assets().len();
            if count >= 1 {
                CheckResult::new(name, Status::Pass, format!("{count} enabled asset(s)"))
            } else {
                CheckResult::new(name, Status::Fail, "no enabled assets")
            }
        }
        Err(err) => CheckResult::new(name, Status::Fail, format!("load failed: {err}")),
    }
}

/// Check 4: the data directory exists (or can be created) and is writable.
fn check_data_dir(dir: &str) -> CheckResult {
    let name = format!("data dir: {dir}");
    if let Err(err) = std::fs::create_dir_all(dir) {
        return CheckResult::new(name, Status::Fail, format!("create failed: {err}"));
    }
    let probe = std::path::Path::new(dir).join(".guardrail-doctor-probe");
    if let Err(err) = std::fs::write(&probe, b"ok") {
        return CheckResult::new(name, Status::Fail, format!("write failed: {err}"));
    }
    if let Err(err) = std::fs::remove_file(&probe) {
        return CheckResult::new(name, Status::Fail, format!("cleanup failed: {err}"));
    }
    CheckResult::new(name, Status::Pass, "exists and writable")
}

/// Check 5: every symbol in a risk policy's `allowed_assets` is present and
/// enabled in the eligible universe. This guards the exact drift that once broke
/// live trading — the policy allowing a symbol the universe no longer ships.
fn check_allowlist_subset(policy_path: &str, universe_path: &str) -> CheckResult {
    let name = format!("allowlist ⊆ universe: {policy_path}");

    let universe = match Universe::load(universe_path) {
        Ok(u) => u,
        Err(err) => {
            return CheckResult::new(name, Status::Fail, format!("universe load failed: {err}"));
        }
    };
    let raw = match std::fs::read_to_string(policy_path) {
        Ok(raw) => raw,
        Err(err) => return CheckResult::new(name, Status::Fail, format!("read failed: {err}")),
    };
    let policy = match RiskPolicy::from_json_str(&raw) {
        Ok(policy) => policy,
        Err(err) => return CheckResult::new(name, Status::Fail, format!("parse failed: {err}")),
    };

    // An empty allowlist means "any eligible asset" — nothing to cross-check.
    if policy.allowed_assets.is_empty() {
        return CheckResult::new(
            name,
            Status::Warn,
            "allowlist empty (allows any eligible asset)",
        );
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
            format!(
                "all {} allowed asset(s) are eligible",
                policy.allowed_assets.len()
            ),
        )
    } else {
        CheckResult::new(
            name,
            Status::Fail,
            format!("not in universe: {}", missing.join(", ")),
        )
    }
}

/// Check 6: the latest run report exists and parses (informational — absence is
/// a warn, since a fresh checkout has not run the agent yet).
fn check_run_report(path: &str) -> CheckResult {
    let name = format!("run report: {path}");
    if !std::path::Path::new(path).exists() {
        return CheckResult::new(name, Status::Warn, "absent (agent has not run yet)");
    }
    match std::fs::read_to_string(path).map(|raw| serde_json::from_str::<serde_json::Value>(&raw)) {
        Ok(Ok(value)) => {
            let cycles = value.get("cycles").and_then(|c| c.as_u64());
            let detail = match cycles {
                Some(n) => format!("valid; {n} cycle(s) recorded"),
                None => "valid JSON".to_string(),
            };
            CheckResult::new(name, Status::Pass, detail)
        }
        Ok(Err(err)) => CheckResult::new(name, Status::Fail, format!("invalid JSON: {err}")),
        Err(err) => CheckResult::new(name, Status::Fail, format!("read failed: {err}")),
    }
}

const RUN_REPORT_FILE: &str = "data/run_report.json";

/// Run every check and collect results in order.
fn run_checks() -> Vec<CheckResult> {
    let mut results = Vec::new();

    for path in CONFIG_FILES {
        results.push(check_config(path));
    }
    for path in RISK_POLICY_FILES {
        results.push(check_risk_policy(path));
    }
    results.push(check_universe(UNIVERSE_FILE));
    for path in RISK_POLICY_FILES {
        results.push(check_allowlist_subset(path, UNIVERSE_FILE));
    }
    results.push(check_data_dir(DATA_DIR));
    results.push(check_run_report(RUN_REPORT_FILE));

    results
}

/// Print a checklist table and the final READY / NOT READY summary.
/// Returns true if every check passed (no failures).
fn report(results: &[CheckResult]) -> bool {
    let name_width = results
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(0)
        .max("CHECK".len());

    println!();
    println!("Guardrail Doctor \u{2014} Preflight Checks");
    println!("{}", "=".repeat(name_width + 30));
    println!("  {:<width$}  STATUS  DETAIL", "CHECK", width = name_width);
    println!("{}", "-".repeat(name_width + 30));

    let mut failures = 0usize;
    let mut warnings = 0usize;

    for result in results {
        match result.status {
            Status::Fail => failures += 1,
            Status::Warn => warnings += 1,
            Status::Pass => {}
        }
        println!(
            "{} {:<width$}  {:<6}  {}",
            result.status.mark(),
            result.name,
            match result.status {
                Status::Pass => "PASS",
                Status::Warn => "WARN",
                Status::Fail => "FAIL",
            },
            result.detail,
            width = name_width,
        );
    }

    println!("{}", "-".repeat(name_width + 30));
    println!(
        "Summary: {} passed, {} warned, {} failed (of {} checks)",
        results.len() - failures - warnings,
        warnings,
        failures,
        results.len(),
    );

    let ready = failures == 0;
    if ready {
        println!("\nREADY");
    } else {
        println!("\nNOT READY");
    }
    println!();

    ready
}

/// Emit the checks as a JSON document (for CI / dashboards).
fn report_json(results: &[CheckResult]) -> bool {
    let failures = results.iter().filter(|r| r.status == Status::Fail).count();
    let warnings = results.iter().filter(|r| r.status == Status::Warn).count();
    let ready = failures == 0;

    let checks: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": r.name,
                "status": match r.status {
                    Status::Pass => "pass",
                    Status::Warn => "warn",
                    Status::Fail => "fail",
                },
                "detail": r.detail,
            })
        })
        .collect();

    let doc = serde_json::json!({
        "ready": ready,
        "summary": {
            "total": results.len(),
            "passed": results.len() - failures - warnings,
            "warned": warnings,
            "failed": failures,
        },
        "checks": checks,
    });
    println!("{}", serde_json::to_string_pretty(&doc).unwrap_or_default());
    ready
}

fn main() {
    let json = std::env::args().any(|a| a == "--json");
    let results = run_checks();
    let ready = if json {
        report_json(&results)
    } else {
        report(&results)
    };
    if !ready {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Resolve a repo-relative config path independently of the test runner's
    /// working directory (cargo runs tests from the crate dir, not repo root).
    fn repo_path(rel: &str) -> String {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(rel);
        root.to_string_lossy().into_owned()
    }

    #[test]
    fn allowlist_subset_holds_for_repo_policies() {
        // The repo's own policies must be a subset of the eligible universe.
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

    #[test]
    fn status_marks_are_distinct() {
        assert_ne!(Status::Pass.mark(), Status::Fail.mark());
        assert_ne!(Status::Pass.mark(), Status::Warn.mark());
    }
}
