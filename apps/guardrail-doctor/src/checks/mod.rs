//! Preflight checks, grouped by domain. `run_checks` runs them all in order.

pub mod config;
pub mod data;
pub mod policy;
pub mod skills;
pub mod universe;

use crate::check::CheckResult;

pub const CONFIG_FILES: [&str; 3] = [
    "configs/paper.toml",
    "configs/production.toml",
    "configs/backtest.toml",
];

pub const RISK_POLICY_FILES: [&str; 2] = [
    "configs/risk_policy.paper.json",
    "configs/risk_policy.production.json",
];

pub const UNIVERSE_FILE: &str = "configs/eligible_assets.bsc.json";
pub const DATA_DIR: &str = "data";
pub const MIGRATIONS_DIR: &str = "migrations";
pub const RUN_REPORT_FILE: &str = "data/run_report.json";
pub const SKILLS_INDEX_FILE: &str = "skills/INDEX.json";
pub const ENSEMBLE_FILE: &str = "skills/ensemble.json";
pub const STRATEGY_PRESETS_FILE: &str = "configs/strategy_presets.json";

/// Run every check and collect results in order.
pub fn run_checks() -> Vec<CheckResult> {
    let mut results = Vec::new();

    for path in CONFIG_FILES {
        results.push(config::check_config(path));
    }
    for path in RISK_POLICY_FILES {
        results.push(policy::check_risk_policy(path));
    }
    results.push(universe::check_universe(UNIVERSE_FILE));
    for path in RISK_POLICY_FILES {
        results.push(policy::check_allowlist_subset(path, UNIVERSE_FILE));
    }
    results.push(skills::check_skills_index(SKILLS_INDEX_FILE));
    results.push(skills::check_ensemble_weights(ENSEMBLE_FILE));
    results.push(skills::check_strategy_presets(STRATEGY_PRESETS_FILE));
    results.push(data::check_migrations(MIGRATIONS_DIR));
    results.push(data::check_data_dir(DATA_DIR));
    results.push(data::check_run_report(RUN_REPORT_FILE));

    results
}
