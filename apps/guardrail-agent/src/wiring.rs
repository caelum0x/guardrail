//! Startup wiring resolution for the Guardrail agent binary.
//!
//! This module mirrors the transport-selection logic in `agent_runtime`
//! (`build_data_source` / `build_executor`) *without* constructing any clients,
//! so the binary can print an accurate, human-readable summary of exactly how
//! the trading loop will be wired before it starts. It reads the same config
//! fields and the same `CMC_API_KEY` / `TWAK_BASE_URL` environment variables the
//! runtime consults, so the reported wiring matches the runtime's real choices.

use common::Settings;
use std::fmt;

/// The market-data transport the runtime will resolve from config + env.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSourceKind {
    /// Deterministic, network-free mock (paper-mode default).
    Mock,
    /// Live CoinMarketCap JSON-RPC MCP endpoint.
    Mcp,
    /// Live CoinMarketCap REST API (keyed by `CMC_API_KEY`).
    Rest,
}

impl DataSourceKind {
    fn label(self) -> &'static str {
        match self {
            DataSourceKind::Mock => "mock (offline, deterministic)",
            DataSourceKind::Mcp => "live CMC MCP",
            DataSourceKind::Rest => "live CMC REST",
        }
    }

    /// Whether this transport reaches the network.
    pub fn is_live(self) -> bool {
        !matches!(self, DataSourceKind::Mock)
    }
}

/// The TWAK execution transport the runtime will resolve from `twak.mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutorKind {
    /// Offline mock executor (paper-mode default / unknown-mode fallback).
    Mock,
    /// REST execution surface.
    Rest,
    /// JSON-RPC MCP execution surface.
    Mcp,
    /// Local TWAK CLI executor.
    Cli,
}

impl ExecutorKind {
    fn label(self) -> &'static str {
        match self {
            ExecutorKind::Mock => "mock (offline)",
            ExecutorKind::Rest => "TWAK REST",
            ExecutorKind::Mcp => "TWAK MCP",
            ExecutorKind::Cli => "TWAK CLI",
        }
    }

    /// Whether this executor performs out-of-process signing/execution.
    pub fn is_live(self) -> bool {
        !matches!(self, ExecutorKind::Mock)
    }
}

/// A resolved, human-readable summary of how the agent will be wired.
#[derive(Debug, Clone)]
pub struct WiringSummary {
    pub profile: &'static str,
    pub mode: String,
    pub data_source: DataSourceKind,
    pub executor: ExecutorKind,
    pub quote_before_swap: bool,
    pub autonomous: bool,
    pub competition_register: bool,
    pub loop_interval_seconds: u64,
}

impl WiringSummary {
    /// Resolve the wiring from settings + environment, mirroring the runtime's
    /// `build_data_source` / `build_executor` selection rules exactly.
    pub fn resolve(s: &Settings) -> Self {
        WiringSummary {
            profile: wiring_profile(&s.app.mode),
            mode: s.app.mode.clone(),
            data_source: resolve_data_source(s),
            executor: resolve_executor(s),
            quote_before_swap: s.twak.quote_before_swap,
            autonomous: s.twak.autonomous,
            competition_register: s.twak.competition_register_enabled,
            loop_interval_seconds: s.strategy.loop_interval_seconds,
        }
    }

    /// True when either side of the wiring reaches the network — used to gate
    /// extra confirmation/preflight in live mode.
    pub fn is_live_wiring(&self) -> bool {
        self.data_source.is_live() || self.executor.is_live()
    }
}

impl fmt::Display for WiringSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "guardrail-agent wiring [{}]", self.profile)?;
        writeln!(f, "  mode:              {}", self.mode)?;
        writeln!(f, "  data source:       {}", self.data_source.label())?;
        writeln!(f, "  executor:          {}", self.executor.label())?;
        writeln!(f, "  quote-before-swap: {}", self.quote_before_swap)?;
        writeln!(f, "  autonomous:        {}", self.autonomous)?;
        writeln!(f, "  register:          {}", self.competition_register)?;
        write!(f, "  loop interval:     {}s", self.loop_interval_seconds)
    }
}

/// Mirror of `agent_runtime::build_data_source` selection (resolution only).
fn resolve_data_source(s: &Settings) -> DataSourceKind {
    let api_key = std::env::var("CMC_API_KEY").unwrap_or_default();
    let mcp_url = s.cmc.mcp_url.clone().unwrap_or_default();

    if s.cmc.use_mock {
        DataSourceKind::Mock
    } else if s.cmc.use_mcp && !mcp_url.is_empty() {
        DataSourceKind::Mcp
    } else if !api_key.is_empty() {
        DataSourceKind::Rest
    } else {
        DataSourceKind::Mock
    }
}

/// Mirror of `agent_runtime::build_executor` selection (resolution only).
fn resolve_executor(s: &Settings) -> ExecutorKind {
    match s.twak.mode.as_str() {
        "rest" => ExecutorKind::Rest,
        "mcp" => ExecutorKind::Mcp,
        "cli" => ExecutorKind::Cli,
        _ => ExecutorKind::Mock,
    }
}

/// Human-readable profile label for a run mode.
pub fn wiring_profile(mode: &str) -> &'static str {
    match mode {
        "live" => "live",
        "backtest" => "backtest",
        _ => "paper",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_transports_are_not_live() {
        assert!(!DataSourceKind::Mock.is_live());
        assert!(DataSourceKind::Rest.is_live());
        assert!(!ExecutorKind::Mock.is_live());
        assert!(ExecutorKind::Cli.is_live());
    }

    #[test]
    fn profile_maps_modes() {
        assert_eq!(wiring_profile("live"), "live");
        assert_eq!(wiring_profile("backtest"), "backtest");
        assert_eq!(wiring_profile("paper"), "paper");
        assert_eq!(wiring_profile("anything-else"), "paper");
    }
}
