use crate::ids;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Direction of an order relative to the risk asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    /// Increase exposure (stable -> risk asset).
    Buy,
    /// Reduce exposure (risk asset -> stable).
    Sell,
}

/// A strategy's proposal to trade. This is *intent only* — it carries no
/// authority. It must pass the risk engine and be quoted before execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderIntent {
    pub id: String,
    pub side: OrderSide,
    pub from_symbol: String,
    pub to_symbol: String,
    pub amount_usd: Decimal,
    pub reason: String,
}

impl OrderIntent {
    pub fn new(
        side: OrderSide,
        from_symbol: impl Into<String>,
        to_symbol: impl Into<String>,
        amount_usd: Decimal,
        reason: impl Into<String>,
    ) -> Self {
        OrderIntent {
            id: ids::new_id(),
            side,
            from_symbol: from_symbol.into(),
            to_symbol: to_symbol.into(),
            amount_usd,
            reason: reason.into(),
        }
    }
}

/// A desired portfolio weight for a symbol, expressed as a percent of NAV.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetPosition {
    pub symbol: String,
    pub weight_pct: Decimal,
}

/// Minimal quote facts the risk engine needs for its final pre-execution check.
///
/// Defined here (not in `twak-client`) so the risk engine never depends on the
/// executor — it only reasons over numbers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteSummary {
    pub expected_out_usd: Decimal,
    pub price_impact_pct: Decimal,
    pub slippage_pct: Decimal,
    pub liquidity_usd: Decimal,
}
