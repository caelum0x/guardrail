//! Trades produced by the matching engine.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A fill between an incoming (taker) order and a resting (maker) order.
///
/// Trades always execute at the resting maker's price (price improvement
/// accrues to the taker), per standard continuous-auction matching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trade {
    pub taker_id: u64,
    pub maker_id: u64,
    pub price: Decimal,
    pub quantity: Decimal,
}

impl Trade {
    /// Notional value of the fill (`price * quantity`).
    #[inline]
    pub fn notional(&self) -> Decimal {
        self.price * self.quantity
    }
}
