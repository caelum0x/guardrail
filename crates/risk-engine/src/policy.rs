use common::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyTradeRequirement {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_min_trades")]
    pub min_trades_per_day: u32,
    #[serde(default = "default_heartbeat_pct")]
    pub max_heartbeat_trade_pct: Decimal,
}

impl Default for DailyTradeRequirement {
    fn default() -> Self {
        Self {
            enabled: true,
            min_trades_per_day: default_min_trades(),
            max_heartbeat_trade_pct: default_heartbeat_pct(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskPolicy {
    pub max_total_drawdown_pct: Decimal,
    pub max_daily_drawdown_pct: Decimal,
    pub max_position_pct: Decimal,
    pub max_new_position_pct: Decimal,
    pub min_stable_reserve_pct: Decimal,
    pub max_slippage_pct: Decimal,
    pub kill_switch_drawdown_pct: Decimal,
    #[serde(default)]
    pub allowed_assets: Vec<String>,
    #[serde(default)]
    pub allowed_chains: Vec<u64>,
    #[serde(default)]
    pub execution_layer: String,
    #[serde(default = "default_true")]
    pub require_quote_before_swap: bool,
    #[serde(default)]
    pub daily_trade_requirement: DailyTradeRequirement,
    #[serde(default)]
    pub forbidden_actions: Vec<String>,
}

impl Default for RiskPolicy {
    fn default() -> Self {
        Self {
            max_total_drawdown_pct: dec!(22),
            max_daily_drawdown_pct: dec!(7),
            max_position_pct: dec!(18),
            max_new_position_pct: dec!(12),
            min_stable_reserve_pct: dec!(10),
            max_slippage_pct: dec!(0.8),
            kill_switch_drawdown_pct: dec!(24),
            allowed_assets: vec!["USDT".into(), "CAKE".into(), "WBNB".into()],
            allowed_chains: vec![56],
            execution_layer: "twak_only".into(),
            require_quote_before_swap: true,
            daily_trade_requirement: DailyTradeRequirement::default(),
            forbidden_actions: vec![
                "launch_token".into(),
                "borrow_without_policy".into(),
                "custodial_signing".into(),
                "trade_non_eligible_assets".into(),
                "bypass_twak".into(),
            ],
        }
    }
}

impl RiskPolicy {
    pub fn from_json_str(input: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(input)?)
    }

    pub fn asset_allowed(&self, symbol: &str) -> bool {
        self.allowed_assets.is_empty() || self.allowed_assets.iter().any(|s| s == symbol)
    }
}

fn default_true() -> bool {
    true
}

fn default_min_trades() -> u32 {
    1
}

fn default_heartbeat_pct() -> Decimal {
    dec!(2)
}
