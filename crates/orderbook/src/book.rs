//! The order book and its price-time-priority matching engine.

use std::collections::{BTreeMap, VecDeque};

use rust_decimal::Decimal;

use crate::order::{Order, OrderKind, Side};
use crate::trade::Trade;

/// Aggregated top-of-book liquidity, one `(price, total_quantity)` pair per
/// level, best price first on each side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookDepth {
    pub bids: Vec<(Decimal, Decimal)>,
    pub asks: Vec<(Decimal, Decimal)>,
}

/// An in-memory limit order book.
///
/// Each side maps a price level to a FIFO queue of resting orders, so matching
/// honors price priority (best price first) then time priority (oldest order at
/// a level first). Bids are best at the highest price; asks at the lowest.
#[derive(Debug, Default)]
pub struct OrderBook {
    bids: BTreeMap<Decimal, VecDeque<Order>>,
    asks: BTreeMap<Decimal, VecDeque<Order>>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Submit an order. It is matched against the opposite side per price-time
    /// priority and the resulting fills are returned. A limit order's unfilled
    /// remainder rests in the book; a market order never rests (any unfilled
    /// remainder is discarded — there was no more liquidity to take).
    pub fn submit(&mut self, mut order: Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        if order.quantity <= Decimal::ZERO {
            return trades;
        }
        match order.side {
            Side::Buy => self.match_against_asks(&mut order, &mut trades),
            Side::Sell => self.match_against_bids(&mut order, &mut trades),
        }
        if !order.is_filled() {
            if let OrderKind::Limit { price } = order.kind {
                let side = match order.side {
                    Side::Buy => &mut self.bids,
                    Side::Sell => &mut self.asks,
                };
                side.entry(price).or_default().push_back(order);
            }
        }
        trades
    }

    /// A buy taker lifts asks from the lowest price up, stopping at its limit.
    fn match_against_asks(&mut self, order: &mut Order, trades: &mut Vec<Trade>) {
        let limit = order.kind.limit_price();
        while !order.is_filled() {
            let Some(&best) = self.asks.keys().next() else {
                break;
            };
            if let Some(limit_price) = limit {
                if best > limit_price {
                    break;
                }
            }
            fill_level(self.asks.get_mut(&best).expect("level exists"), order, best, trades);
            if self.asks.get(&best).is_some_and(VecDeque::is_empty) {
                self.asks.remove(&best);
            }
        }
    }

    /// A sell taker hits bids from the highest price down, stopping at its limit.
    fn match_against_bids(&mut self, order: &mut Order, trades: &mut Vec<Trade>) {
        let limit = order.kind.limit_price();
        while !order.is_filled() {
            let Some(&best) = self.bids.keys().next_back() else {
                break;
            };
            if let Some(limit_price) = limit {
                if best < limit_price {
                    break;
                }
            }
            fill_level(self.bids.get_mut(&best).expect("level exists"), order, best, trades);
            if self.bids.get(&best).is_some_and(VecDeque::is_empty) {
                self.bids.remove(&best);
            }
        }
    }

    /// Cancel a resting order by id. Returns true if it was found and removed.
    pub fn cancel(&mut self, id: u64) -> bool {
        for side in [&mut self.bids, &mut self.asks] {
            let mut empties: Vec<Decimal> = Vec::new();
            let mut removed = false;
            for (price, queue) in side.iter_mut() {
                if let Some(pos) = queue.iter().position(|o| o.id == id) {
                    queue.remove(pos);
                    removed = true;
                    if queue.is_empty() {
                        empties.push(*price);
                    }
                    break;
                }
            }
            for price in empties {
                side.remove(&price);
            }
            if removed {
                return true;
            }
        }
        false
    }

    /// Highest bid price, if any.
    pub fn best_bid(&self) -> Option<Decimal> {
        self.bids.keys().next_back().copied()
    }

    /// Lowest ask price, if any.
    pub fn best_ask(&self) -> Option<Decimal> {
        self.asks.keys().next().copied()
    }

    /// Best ask minus best bid, if both sides have liquidity.
    pub fn spread(&self) -> Option<Decimal> {
        Some(self.best_ask()? - self.best_bid()?)
    }

    /// Aggregated depth: up to `levels` price levels per side, best first.
    pub fn depth(&self, levels: usize) -> BookDepth {
        let sum = |q: &VecDeque<Order>| q.iter().map(|o| o.quantity).sum::<Decimal>();
        let bids = self
            .bids
            .iter()
            .rev()
            .take(levels)
            .map(|(p, q)| (*p, sum(q)))
            .collect();
        let asks = self
            .asks
            .iter()
            .take(levels)
            .map(|(p, q)| (*p, sum(q)))
            .collect();
        BookDepth { bids, asks }
    }

    /// Total number of resting orders across both sides.
    pub fn len(&self) -> usize {
        let count = |b: &BTreeMap<Decimal, VecDeque<Order>>| b.values().map(VecDeque::len).sum::<usize>();
        count(&self.bids) + count(&self.asks)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Fill an incoming order against one resting price level (a FIFO queue) at the
/// resting `price`. Fully-filled makers are popped; the incoming order and the
/// makers are decremented in place.
fn fill_level(queue: &mut VecDeque<Order>, order: &mut Order, price: Decimal, trades: &mut Vec<Trade>) {
    while !order.is_filled() {
        let Some(maker) = queue.front_mut() else {
            break;
        };
        let fill = order.quantity.min(maker.quantity);
        trades.push(Trade {
            taker_id: order.id,
            maker_id: maker.id,
            price,
            quantity: fill,
        });
        order.quantity -= fill;
        maker.quantity -= fill;
        if maker.is_filled() {
            queue.pop_front();
        }
    }
}
