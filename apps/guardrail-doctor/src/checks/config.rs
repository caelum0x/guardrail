//! Config-file load checks.

use common::Settings;

use crate::check::{CheckResult, Status};

/// Each config loads via `Settings::load`. Missing files are a warn, not a fail.
pub fn check_config(path: &str) -> CheckResult {
    let name = format!("config: {path}");
    if !std::path::Path::new(path).exists() {
        return CheckResult::warn(name, "file absent (skipped)");
    }
    match Settings::load(path) {
        Ok(settings) => CheckResult::new(
            name,
            Status::Pass,
            format!("loaded; risk.policy_path = {}", settings.risk.policy_path),
        ),
        Err(err) => CheckResult::fail(name, format!("load failed: {err}")),
    }
}
