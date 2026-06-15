//! Eligible-asset universe check.

use market_data::Universe;

use crate::check::CheckResult;

/// The eligible-asset universe loads and has at least one enabled asset.
pub fn check_universe(path: &str) -> CheckResult {
    let name = format!("universe: {path}");
    match Universe::load(path) {
        Ok(universe) => {
            let count = universe.enabled_assets().len();
            if count >= 1 {
                CheckResult::pass(name, format!("{count} enabled asset(s)"))
            } else {
                CheckResult::fail(name, "no enabled assets")
            }
        }
        Err(err) => CheckResult::fail(name, format!("load failed: {err}")),
    }
}
