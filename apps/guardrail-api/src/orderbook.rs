//! Order-book matching demo endpoint: `GET /orderbook`.
//!
//! Builds an in-memory book from a compact order spec, runs the real
//! `orderbook` matching engine, and returns the resulting trades plus the
//! final top-of-book and aggregated depth. Read-only and pure — nothing is
//! persisted; this exercises the matching engine over caller-supplied orders.
//!
//! `?orders=` is a `;`-separated list of `side,kind,price,qty` entries, where
//! `side` is `b`/`buy` or `s`/`sell` and `kind` is `limit` (needs a price) or
//! `market` (price ignored). Example:
//! `orders=b,limit,100,5;s,limit,101,3;b,market,,4`

use std::str::FromStr;

use axum::extract::Query;
use axum::Json;
use orderbook::{Order, OrderBook, Side};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct OrderbookQuery {
    orders: Option<String>,
}

const DEFAULT_ORDERS: &str = "s,limit,101,5;s,limit,102,5;b,limit,99,5;b,market,,6";

fn parse_orders(spec: &str) -> Result<Vec<Order>, String> {
    let mut out = Vec::new();
    for (i, raw) in spec.split(';').filter(|s| !s.trim().is_empty()).enumerate() {
        let parts: Vec<&str> = raw.split(',').map(str::trim).collect();
        if parts.len() != 4 {
            return Err(format!("order {i}: expected 'side,kind,price,qty', got '{raw}'"));
        }
        let side = match parts[0].to_lowercase().as_str() {
            "b" | "buy" => Side::Buy,
            "s" | "sell" => Side::Sell,
            other => return Err(format!("order {i}: bad side '{other}'")),
        };
        let qty = Decimal::from_str(parts[3]).map_err(|_| format!("order {i}: bad qty '{}'", parts[3]))?;
        let id = (i + 1) as u64;
        let ts = i as u64;
        let order = match parts[1].to_lowercase().as_str() {
            "market" => Order::market(id, side, qty, ts),
            "limit" => {
                let price =
                    Decimal::from_str(parts[2]).map_err(|_| format!("order {i}: bad price '{}'", parts[2]))?;
                Order::limit(id, side, price, qty, ts)
            }
            other => return Err(format!("order {i}: bad kind '{other}'")),
        };
        out.push(order);
    }
    Ok(out)
}

pub async fn orderbook(Query(q): Query<OrderbookQuery>) -> Json<Value> {
    let spec = q.orders.unwrap_or_else(|| DEFAULT_ORDERS.to_string());
    let orders = match parse_orders(&spec) {
        Ok(orders) => orders,
        Err(e) => return Json(json!({ "error": e, "format": "side,kind,price,qty;… e.g. b,limit,100,5;s,market,,3" })),
    };

    let mut book = OrderBook::new();
    let mut all_trades: Vec<Value> = Vec::new();
    for order in orders {
        for t in book.submit(order) {
            all_trades.push(json!({
                "taker_id": t.taker_id,
                "maker_id": t.maker_id,
                "price": t.price.to_string(),
                "quantity": t.quantity.to_string(),
            }));
        }
    }

    let depth = book.depth(5);
    let levels = |v: &[(Decimal, Decimal)]| -> Vec<Value> {
        v.iter().map(|(p, q)| json!({ "price": p.to_string(), "quantity": q.to_string() })).collect()
    };

    Json(json!({
        "spec": spec,
        "trades": all_trades,
        "trade_count": all_trades.len(),
        "best_bid": book.best_bid().map(|d| d.to_string()),
        "best_ask": book.best_ask().map(|d| d.to_string()),
        "spread": book.spread().map(|d| d.to_string()),
        "depth": { "bids": levels(&depth.bids), "asks": levels(&depth.asks) },
        "resting_orders": book.len(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_mixed_spec() {
        let orders = parse_orders("b,limit,100,5;s,market,,3").unwrap();
        assert_eq!(orders.len(), 2);
    }

    #[test]
    fn rejects_bad_spec() {
        assert!(parse_orders("b,limit,100").is_err());
        assert!(parse_orders("x,limit,100,5").is_err());
    }
}
