//! Strongly-typed runtime configuration, loaded from the TOML files in
//! `configs/` and overridable by environment variables.

use crate::error::{CommonError, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub app: AppCfg,
    pub chain: ChainCfg,
    pub cmc: CmcCfg,
    pub twak: TwakCfg,
    pub strategy: StrategyCfg,
    pub risk: RiskCfg,
    #[serde(default)]
    pub reporting: ReportingCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppCfg {
    pub name: String,
    /// "live" | "paper" | "backtest"
    pub mode: String,
    pub database_url: String,
}

impl AppCfg {
    pub fn is_live(&self) -> bool {
        self.mode == "live"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainCfg {
    pub chain_id: u64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmcCfg {
    #[serde(default)]
    pub use_rest: bool,
    #[serde(default)]
    pub use_mcp: bool,
    #[serde(default)]
    pub use_x402: bool,
    #[serde(default = "default_timeout")]
    pub request_timeout_ms: u64,
    /// When true the agent uses the deterministic mock data source.
    #[serde(default)]
    pub use_mock: bool,
    /// JSON-RPC 2.0 MCP endpoint URL, used when `use_mcp` is enabled.
    #[serde(default)]
    pub mcp_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwakCfg {
    /// "mcp" | "rest" | "cli" | "mock"
    pub mode: String,
    #[serde(default = "default_true")]
    pub quote_before_swap: bool,
    #[serde(default)]
    pub competition_register_enabled: bool,
    /// Allow the executor to self-submit swaps on the REST surface.
    #[serde(default)]
    pub autonomous: bool,
    /// Base URL for the REST/MCP execution surfaces.
    #[serde(default)]
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyCfg {
    pub loop_interval_seconds: u64,
    pub rebalance_threshold_pct: Decimal,
    pub max_positions: u32,
    pub min_score_to_enter: f64,
    pub min_score_to_hold: f64,
    /// Force-exit a position once its unrealized loss breaches this percent.
    #[serde(default = "default_stop_loss_pct")]
    pub stop_loss_pct: f64,
    /// Force-exit a position once its unrealized gain breaches this percent.
    #[serde(default = "default_take_profit_pct")]
    pub take_profit_pct: f64,
    /// Target share of the book to hold in the stable reserve.
    #[serde(default = "default_target_stable_reserve_pct")]
    pub target_stable_reserve_pct: f64,
    /// Allocation method used to size risk positions. One of
    /// "equal_weight" | "score_proportional" | "inverse_volatility" | "risk_parity".
    #[serde(default = "default_allocation_method")]
    pub allocation_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskCfg {
    pub policy_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportingCfg {
    #[serde(default)]
    pub daily_report_enabled: bool,
    #[serde(default)]
    pub publish_report_hash: bool,
}

fn default_timeout() -> u64 {
    8000
}

fn default_stop_loss_pct() -> f64 {
    12.0
}

fn default_take_profit_pct() -> f64 {
    25.0
}

fn default_target_stable_reserve_pct() -> f64 {
    15.0
}

fn default_allocation_method() -> String {
    "score_proportional".to_string()
}

fn default_true() -> bool {
    true
}

impl Settings {
    /// Load settings from a TOML file, then overlay any `GUARDRAIL_*`
    /// environment variables (e.g. `GUARDRAIL_APP__MODE=paper`).
    pub fn load(path: &str) -> Result<Self> {
        let builder = config::Config::builder()
            .add_source(config::File::with_name(path))
            .add_source(
                config::Environment::with_prefix("GUARDRAIL")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()
            .map_err(|e| CommonError::Config(e.to_string()))?;

        builder
            .try_deserialize()
            .map_err(|e| CommonError::Config(e.to_string()))
    }
}
