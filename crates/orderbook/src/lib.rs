//! In-memory limit order book with a price-time-priority matching engine.
//!
//! ```
//! use orderbook::{OrderBook, Order, Side};
//! use rust_decimal::Decimal;
//!
//! let mut book = OrderBook::new();
//! // Rest an ask, then cross it with a buy.
//! let _ = book.submit(Order::limit(1, Side::Sell, Decimal::from(100), Decimal::from(5), 1));
//! let trades = book.submit(Order::limit(2, Side::Buy, Decimal::from(101), Decimal::from(3), 2));
//! assert_eq!(trades.len(), 1);
//! assert_eq!(trades[0].price, Decimal::from(100)); // executes at the maker price
//! assert_eq!(trades[0].quantity, Decimal::from(3));
//! ```

mod book;
mod order;
mod trade;

pub use book::{BookDepth, OrderBook};
pub use order::{Order, OrderKind, Side};
pub use trade::Trade;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn d(n: i64) -> Decimal {
        Decimal::from(n)
    }

    #[test]
    fn limit_orders_rest_and_set_top_of_book() {
        let mut book = OrderBook::new();
        assert!(book.submit(Order::limit(1, Side::Buy, d(99), d(10), 1)).is_empty());
        assert!(book.submit(Order::limit(2, Side::Sell, d(101), d(10), 2)).is_empty());
        assert_eq!(book.best_bid(), Some(d(99)));
        assert_eq!(book.best_ask(), Some(d(101)));
        assert_eq!(book.spread(), Some(d(2)));
        assert_eq!(book.len(), 2);
    }

    #[test]
    fn crossing_limit_fills_at_maker_price() {
        let mut book = OrderBook::new();
        book.submit(Order::limit(1, Side::Sell, d(100), d(5), 1));
        // Buyer willing to pay 105 lifts the 100 ask; trade executes at 100.
        let trades = book.submit(Order::limit(2, Side::Buy, d(105), d(5), 2));
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, d(100));
        assert_eq!(trades[0].quantity, d(5));
        assert!(book.is_empty(), "both orders fully filled");
    }

    #[test]
    fn partial_fill_rests_remainder() {
        let mut book = OrderBook::new();
        book.submit(Order::limit(1, Side::Sell, d(100), d(3), 1));
        let trades = book.submit(Order::limit(2, Side::Buy, d(100), d(10), 2));
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, d(3));
        // 7 remaining buy qty rests as the new best bid.
        assert_eq!(book.best_bid(), Some(d(100)));
        let depth = book.depth(1);
        assert_eq!(depth.bids, vec![(d(100), d(7))]);
    }

    #[test]
    fn market_order_sweeps_multiple_levels() {
        let mut book = OrderBook::new();
        book.submit(Order::limit(1, Side::Sell, d(100), d(2), 1));
        book.submit(Order::limit(2, Side::Sell, d(101), d(2), 2));
        book.submit(Order::limit(3, Side::Sell, d(102), d(2), 3));
        let trades = book.submit(Order::market(9, Side::Buy, d(5), 9));
        // 2@100, 2@101, 1@102
        assert_eq!(trades.len(), 3);
        assert_eq!(trades[0].price, d(100));
        assert_eq!(trades[1].price, d(101));
        assert_eq!(trades[2].price, d(102));
        assert_eq!(trades[2].quantity, d(1));
        // One unit left at 102.
        assert_eq!(book.best_ask(), Some(d(102)));
    }

    #[test]
    fn time_priority_within_a_level_is_fifo() {
        let mut book = OrderBook::new();
        book.submit(Order::limit(1, Side::Sell, d(100), d(5), 1)); // first in
        book.submit(Order::limit(2, Side::Sell, d(100), d(5), 2)); // second in
        let trades = book.submit(Order::market(9, Side::Buy, d(5), 9));
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].maker_id, 1, "oldest order at the level fills first");
    }

    #[test]
    fn market_remainder_is_discarded_not_rested() {
        let mut book = OrderBook::new();
        book.submit(Order::limit(1, Side::Sell, d(100), d(2), 1));
        let trades = book.submit(Order::market(9, Side::Buy, d(10), 9));
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, d(2));
        // Market order does not rest; book is empty after taking all liquidity.
        assert!(book.is_empty());
        assert_eq!(book.best_bid(), None);
    }

    #[test]
    fn cancel_removes_resting_order() {
        let mut book = OrderBook::new();
        book.submit(Order::limit(1, Side::Buy, d(99), d(5), 1));
        book.submit(Order::limit(2, Side::Buy, d(98), d(5), 2));
        assert!(book.cancel(1));
        assert_eq!(book.best_bid(), Some(d(98)), "cancel removed the top bid");
        assert!(!book.cancel(1), "second cancel finds nothing");
        assert!(!book.cancel(404));
    }

    #[test]
    fn limit_does_not_cross_when_price_is_unmarketable() {
        let mut book = OrderBook::new();
        book.submit(Order::limit(1, Side::Sell, d(100), d(5), 1));
        // Buyer only willing to pay 99 < 100 ask: no trade, order rests.
        let trades = book.submit(Order::limit(2, Side::Buy, d(99), d(5), 2));
        assert!(trades.is_empty());
        assert_eq!(book.best_bid(), Some(d(99)));
        assert_eq!(book.best_ask(), Some(d(100)));
    }
}
