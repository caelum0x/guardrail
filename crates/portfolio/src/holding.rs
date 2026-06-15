use common::Decimal;
use serde::{Deserialize, Serialize};

/// A single position. Stables are held as a holding too, with price ~= 1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Holding {
    pub symbol: String,
    pub quantity: Decimal,
    /// Volume-weighted average cost per unit, in USD.
    pub avg_cost_usd: Decimal,
    /// Latest mark price per unit, in USD.
    pub price_usd: Decimal,
}

impl Holding {
    pub fn new(symbol: impl Into<String>, quantity: Decimal, price_usd: Decimal) -> Self {
        Holding {
            symbol: symbol.into(),
            quantity,
            avg_cost_usd: price_usd,
            price_usd,
        }
    }

    pub fn market_value_usd(&self) -> Decimal {
        self.quantity * self.price_usd
    }

    pub fn cost_basis_usd(&self) -> Decimal {
        self.quantity * self.avg_cost_usd
    }

    pub fn unrealized_pnl_usd(&self) -> Decimal {
        self.market_value_usd() - self.cost_basis_usd()
    }
}
