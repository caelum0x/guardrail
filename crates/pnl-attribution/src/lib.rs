//! Average-cost realized/unrealized PnL attribution from a stream of fills.
//!
//! Tracks a running position and average cost basis per symbol. A buy raises the
//! position and re-averages the cost; a sell realizes PnL against the average
//! cost for the quantity sold (and reduces the position). Marking open positions
//! to a current price yields unrealized PnL. Trading fees are subtracted from
//! realized PnL. All math is exact (`rust_decimal`).
//!
//! ```
//! use pnl_attribution::{Attributor, Fill, Side};
//! use rust_decimal::Decimal;
//!
//! let mut a = Attributor::new();
//! a.apply(&Fill::new("CAKE", Side::Buy, Decimal::from(10), Decimal::from(2), Decimal::ZERO));
//! a.apply(&Fill::new("CAKE", Side::Sell, Decimal::from(4), Decimal::from(3), Decimal::ZERO));
//! // sold 4 @ 3 against avg cost 2 => realized 4
//! let report = a.report(&Default::default());
//! assert_eq!(report.total.realized, Decimal::from(4));
//! ```

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Order side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Buy,
    Sell,
}

/// A single executed fill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fill {
    pub symbol: String,
    pub side: Side,
    pub quantity: Decimal,
    pub price: Decimal,
    /// Fee paid on this fill, in quote currency (subtracted from realized PnL).
    pub fee: Decimal,
}

impl Fill {
    pub fn new(
        symbol: impl Into<String>,
        side: Side,
        quantity: Decimal,
        price: Decimal,
        fee: Decimal,
    ) -> Self {
        Fill {
            symbol: symbol.into(),
            side,
            quantity,
            price,
            fee,
        }
    }
}

/// Running state for one symbol.
#[derive(Debug, Clone, Default)]
struct Lot {
    position: Decimal,
    avg_cost: Decimal,
    realized: Decimal,
    fees: Decimal,
}

/// PnL for a single symbol (or the portfolio total).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PnL {
    pub symbol: String,
    pub position: Decimal,
    pub avg_cost: Decimal,
    pub realized: Decimal,
    pub unrealized: Decimal,
    pub fees: Decimal,
    pub total: Decimal,
}

/// Full attribution report: per-symbol rows plus a portfolio total.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub by_symbol: Vec<PnL>,
    pub total: PortfolioTotal,
}

/// Aggregated portfolio PnL.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioTotal {
    pub realized: Decimal,
    pub unrealized: Decimal,
    pub fees: Decimal,
    pub total: Decimal,
}

/// Accumulates fills into per-symbol average-cost lots.
#[derive(Debug, Default)]
pub struct Attributor {
    lots: BTreeMap<String, Lot>,
}

impl Attributor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply one fill to the running state.
    pub fn apply(&mut self, fill: &Fill) {
        let lot = self.lots.entry(fill.symbol.clone()).or_default();
        lot.fees += fill.fee;
        match fill.side {
            Side::Buy => {
                // Re-average cost over the combined position.
                let new_position = lot.position + fill.quantity;
                if new_position > Decimal::ZERO {
                    let prev_cost = lot.avg_cost * lot.position;
                    let add_cost = fill.price * fill.quantity;
                    lot.avg_cost = (prev_cost + add_cost) / new_position;
                }
                lot.position = new_position;
            }
            Side::Sell => {
                // Realize PnL against the average cost for the quantity sold.
                let sold = fill.quantity.min(lot.position.max(Decimal::ZERO));
                lot.realized += (fill.price - lot.avg_cost) * sold;
                lot.position -= fill.quantity;
                if lot.position <= Decimal::ZERO {
                    lot.position = lot.position.max(Decimal::ZERO);
                    lot.avg_cost = Decimal::ZERO;
                }
            }
        }
    }

    /// Apply many fills in order.
    pub fn apply_all(&mut self, fills: &[Fill]) {
        for f in fills {
            self.apply(f);
        }
    }

    /// Build the report, marking open positions to `marks` (symbol -> price).
    /// Symbols without a mark contribute zero unrealized PnL.
    pub fn report(&self, marks: &BTreeMap<String, Decimal>) -> Report {
        let mut by_symbol = Vec::new();
        let mut total = PortfolioTotal::default();
        for (symbol, lot) in &self.lots {
            let mark = marks.get(symbol).copied().unwrap_or(lot.avg_cost);
            let unrealized = (mark - lot.avg_cost) * lot.position;
            let net = lot.realized + unrealized - lot.fees;
            total.realized += lot.realized;
            total.unrealized += unrealized;
            total.fees += lot.fees;
            total.total += net;
            by_symbol.push(PnL {
                symbol: symbol.clone(),
                position: lot.position,
                avg_cost: lot.avg_cost,
                realized: lot.realized,
                unrealized,
                fees: lot.fees,
                total: net,
            });
        }
        Report { by_symbol, total }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn marks(pairs: &[(&str, Decimal)]) -> BTreeMap<String, Decimal> {
        pairs.iter().map(|(s, p)| (s.to_string(), *p)).collect()
    }

    #[test]
    fn realizes_pnl_on_sell_against_average_cost() {
        let mut a = Attributor::new();
        a.apply(&Fill::new("CAKE", Side::Buy, dec!(10), dec!(2), dec!(0)));
        a.apply(&Fill::new("CAKE", Side::Sell, dec!(4), dec!(3), dec!(0)));
        let r = a.report(&BTreeMap::new());
        // realized = (3 - 2) * 4 = 4
        assert_eq!(r.total.realized, dec!(4));
        // 6 left at avg cost 2
        assert_eq!(r.by_symbol[0].position, dec!(6));
        assert_eq!(r.by_symbol[0].avg_cost, dec!(2));
    }

    #[test]
    fn averages_cost_across_buys() {
        let mut a = Attributor::new();
        a.apply(&Fill::new("X", Side::Buy, dec!(10), dec!(10), dec!(0)));
        a.apply(&Fill::new("X", Side::Buy, dec!(10), dec!(20), dec!(0)));
        let r = a.report(&BTreeMap::new());
        // avg = (10*10 + 10*20)/20 = 15
        assert_eq!(r.by_symbol[0].avg_cost, dec!(15));
        assert_eq!(r.by_symbol[0].position, dec!(20));
    }

    #[test]
    fn unrealized_marks_open_position() {
        let mut a = Attributor::new();
        a.apply(&Fill::new("X", Side::Buy, dec!(10), dec!(10), dec!(0)));
        let r = a.report(&marks(&[("X", dec!(12))]));
        // (12 - 10) * 10 = 20
        assert_eq!(r.total.unrealized, dec!(20));
        assert_eq!(r.total.total, dec!(20));
    }

    #[test]
    fn fees_reduce_total() {
        let mut a = Attributor::new();
        a.apply(&Fill::new("X", Side::Buy, dec!(10), dec!(10), dec!(5)));
        a.apply(&Fill::new("X", Side::Sell, dec!(10), dec!(11), dec!(5)));
        let r = a.report(&BTreeMap::new());
        // realized = (11-10)*10 = 10 ; fees = 10 ; total = 0
        assert_eq!(r.total.realized, dec!(10));
        assert_eq!(r.total.fees, dec!(10));
        assert_eq!(r.total.total, dec!(0));
    }

    #[test]
    fn multi_symbol_totals_aggregate() {
        let mut a = Attributor::new();
        a.apply(&Fill::new("A", Side::Buy, dec!(1), dec!(100), dec!(0)));
        a.apply(&Fill::new("B", Side::Buy, dec!(2), dec!(50), dec!(0)));
        let r = a.report(&marks(&[("A", dec!(110)), ("B", dec!(40))]));
        // A unrealized = 10 ; B unrealized = (40-50)*2 = -20 ; total = -10
        assert_eq!(r.total.unrealized, dec!(-10));
        assert_eq!(r.by_symbol.len(), 2);
    }
}
