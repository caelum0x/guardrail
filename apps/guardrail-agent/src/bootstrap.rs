//! Pre-flight bootstrap for the Guardrail agent binary.
//!
//! Before the trading loop starts, [`preflight`] validates the resolved
//! configuration and environment so misconfiguration fails fast with a clear
//! message instead of surfacing deep inside the loop. It enforces the safety
//! rules that matter most for a money-moving agent:
//!
//! * the run mode is one of the supported values;
//! * `live` mode is never run against mock data or a mock executor (that would
//!   silently fake a real trading session);
//! * the risk policy file the runtime depends on actually exists;
//! * the SQLite event-log directory exists or can be created.
//!
//! It returns `Err(..)` on a fatal misconfiguration (aborting the binary) and
//! logs warnings for non-fatal concerns.

use crate::wiring::WiringSummary;
use anyhow::{bail, Context};
use common::Settings;
use std::path::Path;

/// The run modes the agent understands.
const SUPPORTED_MODES: [&str; 3] = ["paper", "live", "backtest"];

/// Validate configuration + environment before the runtime starts.
///
/// `config_path` is included in error context so an operator can see which file
/// produced the bad settings. On success the agent is safe to start with the
/// given `wiring`.
pub fn preflight(
    settings: &Settings,
    config_path: &str,
    wiring: &WiringSummary,
) -> anyhow::Result<()> {
    let mode = settings.app.mode.as_str();
    if !SUPPORTED_MODES.contains(&mode) {
        bail!(
            "unsupported app.mode {mode:?} in {config_path} (expected one of {SUPPORTED_MODES:?})"
        );
    }

    // Live mode must never run on faked data or a faked executor.
    if settings.app.is_live() {
        if !wiring.data_source.is_live() {
            bail!(
                "live mode requires a live data source, but wiring resolved to the mock \
                 (set cmc.use_rest/use_mcp and provide CMC_API_KEY / cmc.mcp_url)"
            );
        }
        if !wiring.executor.is_live() {
            bail!(
                "live mode requires a live executor, but twak.mode resolved to mock \
                 (set twak.mode to rest|mcp|cli)"
            );
        }
        if !settings.twak.quote_before_swap {
            tracing::warn!(
                "live mode with quote_before_swap=false — swaps will skip the quote guard"
            );
        }
    } else if wiring.is_live_wiring() {
        // Non-live mode wired to a live transport is suspicious but allowed.
        tracing::warn!(
            mode = %mode,
            "non-live mode is wired to a live transport; no real funds should be at risk"
        );
    }

    // The risk policy file is loaded by the runtime/risk engine; verify it now.
    let policy_path = &settings.risk.policy_path;
    if !Path::new(policy_path).is_file() {
        bail!("risk policy file not found: {policy_path} (configured in {config_path})");
    }

    ensure_database_dir(&settings.app.database_url)
        .with_context(|| format!("preparing event-log location {}", settings.app.database_url))?;

    tracing::info!(
        mode = %mode,
        policy = %policy_path,
        db = %settings.app.database_url,
        "preflight checks passed"
    );
    Ok(())
}

/// Ensure the parent directory of the SQLite database URL exists (creating it if
/// needed). Accepts both bare paths and `sqlite://` URLs; non-file URLs are left
/// alone.
fn ensure_database_dir(database_url: &str) -> anyhow::Result<()> {
    let path = database_url
        .strip_prefix("sqlite://")
        .unwrap_or(database_url);

    // In-memory or non-path URLs need no directory.
    if path.is_empty() || path.starts_with(':') {
        return Ok(());
    }

    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating event-log directory {}", parent.display()))?;
            tracing::info!(dir = %parent.display(), "created event-log directory");
        }
    }
    Ok(())
}

/// Banner printed at startup so logs clearly delimit each run.
pub fn startup_banner(settings: &Settings) -> String {
    format!(
        "Guardrail Alpha · {} · mode={} · chain={} ({})",
        settings.app.name, settings.app.mode, settings.chain.name, settings.chain.chain_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_mode_is_listed() {
        assert!(SUPPORTED_MODES.contains(&"paper"));
        assert!(SUPPORTED_MODES.contains(&"live"));
        assert!(!SUPPORTED_MODES.contains(&"demo"));
    }

    #[test]
    fn in_memory_db_needs_no_dir() {
        assert!(ensure_database_dir(":memory:").is_ok());
        assert!(ensure_database_dir("").is_ok());
    }
}
