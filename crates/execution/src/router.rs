//! Execution routing.
//!
//! Every order is executed through TWAK (the user holds the keys; the agent only
//! requests signed authorizations), but the *venue* TWAK should route to depends
//! on the order: stable<->risk swaps go through the AMM aggregator, while a
//! tiny daily-heartbeat trade can take the cheapest direct pool. This module
//! turns an [`OrderIntent`] into a [`RouteDecision`] the executor can act on.

use common::{Decimal, OrderIntent, OrderSide};

/// The execution venue selected for an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Venue {
    /// AMM aggregator (best price across pools) — the default for real risk
    /// changes.
    Aggregator,
    /// A single direct pool — cheaper gas for tiny/heartbeat-sized orders.
    DirectPool,
}

impl Venue {
    pub fn as_str(self) -> &'static str {
        match self {
            Venue::Aggregator => "twak-aggregator",
            Venue::DirectPool => "twak-direct-pool",
        }
    }
}

/// A routing decision for one order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDecision {
    pub venue: Venue,
    pub from_symbol: String,
    pub to_symbol: String,
    /// Whether this order increases risk exposure (a buy).
    pub increases_exposure: bool,
}

impl RouteDecision {
    /// Stable display name of the route.
    pub fn route_name(&self) -> &'static str {
        self.venue.as_str()
    }
}

/// Orders at or below this USD notional are treated as heartbeat-sized and
/// routed to a single direct pool to save gas.
const HEARTBEAT_NOTIONAL_USD: i64 = 5;

/// Choose a route for an order intent. Large/standard orders go through the
/// aggregator for best execution; dust-sized heartbeat orders take a direct pool.
pub fn route(intent: &OrderIntent) -> RouteDecision {
    let venue = if intent.amount_usd <= Decimal::from(HEARTBEAT_NOTIONAL_USD) {
        Venue::DirectPool
    } else {
        Venue::Aggregator
    };

    RouteDecision {
        venue,
        from_symbol: intent.from_symbol.clone(),
        to_symbol: intent.to_symbol.clone(),
        increases_exposure: intent.side == OrderSide::Buy,
    }
}

/// Backwards-compatible helper retained for existing callers.
pub fn route_name() -> &'static str {
    Venue::Aggregator.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent(side: OrderSide, amount: i64) -> OrderIntent {
        OrderIntent::new(side, "USDT", "WBNB", Decimal::from(amount), "test")
    }

    #[test]
    fn standard_order_uses_aggregator() {
        let d = route(&intent(OrderSide::Buy, 250));
        assert_eq!(d.venue, Venue::Aggregator);
        assert!(d.increases_exposure);
        assert_eq!(d.route_name(), "twak-aggregator");
    }

    #[test]
    fn heartbeat_order_uses_direct_pool() {
        let d = route(&intent(OrderSide::Buy, 3));
        assert_eq!(d.venue, Venue::DirectPool);
    }

    #[test]
    fn sell_does_not_increase_exposure() {
        let d = route(&intent(OrderSide::Sell, 100));
        assert!(!d.increases_exposure);
    }
}
