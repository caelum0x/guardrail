//! Order types for the limit order book.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Side of the market an order sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    /// The opposite side, i.e. the resting side a taker matches against.
    #[inline]
    pub fn opposite(self) -> Side {
        match self {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        }
    }
}

/// Kind of order: a `Limit` order carries a price and may rest in the book,
/// a `Market` order takes whatever liquidity is available and never rests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OrderKind {
    Limit { price: Decimal },
    Market,
}

impl OrderKind {
    /// Returns the limit price if this is a limit order.
    #[inline]
    pub fn limit_price(&self) -> Option<Decimal> {
        match self {
            OrderKind::Limit { price } => Some(*price),
            OrderKind::Market => None,
        }
    }

    #[inline]
    pub fn is_market(&self) -> bool {
        matches!(self, OrderKind::Market)
    }
}

/// A single order submitted to the book.
///
/// `quantity` is the *remaining* quantity; the matching engine decrements it
/// in place on the resting copy as fills occur. Equality and identity are keyed
/// on `id`, which the caller is responsible for keeping unique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Order {
    pub id: u64,
    pub side: Side,
    pub kind: OrderKind,
    /// Remaining quantity. Must be strictly positive when submitted.
    pub quantity: Decimal,
    /// Monotonic submission timestamp used to break price ties (time priority).
    pub timestamp: u64,
}

impl Order {
    /// Construct a limit order.
    pub fn limit(id: u64, side: Side, price: Decimal, quantity: Decimal, timestamp: u64) -> Order {
        Order {
            id,
            side,
            kind: OrderKind::Limit { price },
            quantity,
            timestamp,
        }
    }

    /// Construct a market order.
    pub fn market(id: u64, side: Side, quantity: Decimal, timestamp: u64) -> Order {
        Order {
            id,
            side,
            kind: OrderKind::Market,
            quantity,
            timestamp,
        }
    }

    #[inline]
    pub fn is_filled(&self) -> bool {
        self.quantity <= Decimal::ZERO
    }
}
