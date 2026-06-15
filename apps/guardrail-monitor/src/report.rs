//! Serde model for the agent's run report (`data/run_report.json`).

use serde::{Deserialize, Serialize};

/// A single open position in the portfolio.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Position {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub weight_pct: String,
    #[serde(default)]
    pub value_usd: String,
}

/// A single executed (or attempted) trade.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Trade {
    #[serde(default)]
    pub tx_hash: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub amount_usd: String,
    #[serde(default)]
    pub status: String,
}

/// The full run report emitted by the agent on each cycle.
///
/// Decimal-valued fields are kept as `String` to avoid lossy float parsing;
/// callers parse them into `rust_decimal::Decimal` on demand. All fields use
/// `#[serde(default)]` so a partial report never fails to deserialize.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunReport {
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub updated_ms: i64,
    #[serde(default)]
    pub wallet_address: String,
    #[serde(default)]
    pub nav_usd: String,
    #[serde(default)]
    pub starting_nav_usd: String,
    #[serde(default)]
    pub total_drawdown_pct: String,
    #[serde(default)]
    pub regime: String,
    #[serde(default)]
    pub kill_switch: bool,
    #[serde(default)]
    pub positions: Vec<Position>,
    #[serde(default)]
    pub trades: Vec<Trade>,
    #[serde(default)]
    pub events: i64,
    #[serde(default)]
    pub policy_hash: String,
}
