//! Defensive `serde_json::Value` navigation shared by the REST and MCP surfaces.
//!
//! TWAK responses vary by transport and version, so rather than deriving rigid
//! deserializers we walk the JSON with fallbacks and only synthesize values the
//! shared model types actually expose. Missing fields degrade to sane defaults;
//! a result type is returned only where a field is genuinely required.

use crate::portfolio::{TwakBalance, TwakPortfolio};
use crate::quote::SwapQuote;
use crate::swap::TxReceipt;
use common::{Decimal, OrderIntent, QuoteSummary};
use serde_json::Value;
use std::str::FromStr;

/// Unwrap a common `{ "result": ... }` / `{ "data": ... }` envelope, returning
/// the inner value when present and the original otherwise.
pub fn unwrap_envelope(v: &Value) -> &Value {
    for key in ["result", "data", "payload"] {
        if let Some(inner) = v.get(key) {
            return inner;
        }
    }
    v
}

/// Read a string at the first matching key.
pub fn str_at<'a>(v: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|k| v.get(*k).and_then(Value::as_str))
}

/// Read a `Decimal` from a number or numeric string at the first matching key.
pub fn decimal_at(v: &Value, keys: &[&str]) -> Option<Decimal> {
    keys.iter()
        .find_map(|k| v.get(*k).and_then(value_to_decimal))
}

/// Read a `Decimal` at the first matching key, defaulting to zero.
pub fn decimal_or_zero(v: &Value, keys: &[&str]) -> Decimal {
    decimal_at(v, keys).unwrap_or(Decimal::ZERO)
}

fn value_to_decimal(v: &Value) -> Option<Decimal> {
    match v {
        Value::String(s) => Decimal::from_str(s.trim()).ok(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(Decimal::from(i))
            } else {
                n.as_f64().and_then(Decimal::from_f64_retain)
            }
        }
        _ => None,
    }
}

/// Extract the wallet address from a `/wallet` style response.
pub fn wallet_address(v: &Value) -> Option<String> {
    let inner = unwrap_envelope(v);
    str_at(inner, &["address", "wallet", "wallet_address", "account"])
        .map(str::to_string)
        // Some transports return the bare string.
        .or_else(|| v.as_str().map(str::to_string))
}

/// Build a portfolio from a `/portfolio` style response, tolerating either a
/// top-level array or a `{ balances: [...] }` object.
pub fn portfolio(v: &Value) -> TwakPortfolio {
    let inner = unwrap_envelope(v);
    let items = inner
        .get("balances")
        .and_then(Value::as_array)
        .or_else(|| inner.as_array())
        .cloned()
        .unwrap_or_default();

    let balances = items.iter().filter_map(balance).collect::<Vec<_>>();

    TwakPortfolio { balances }
}

fn balance(v: &Value) -> Option<TwakBalance> {
    let symbol = str_at(v, &["symbol", "asset", "token"])?.to_string();
    Some(TwakBalance {
        symbol,
        amount: decimal_or_zero(v, &["amount", "balance", "quantity"]),
        value_usd: decimal_or_zero(v, &["value_usd", "usd_value", "value"]),
    })
}

/// Build a `SwapQuote` from a `/swap/quote` style response, falling back to the
/// originating intent's symbols/amount where the venue omits them.
pub fn swap_quote(v: &Value, intent: &OrderIntent) -> Option<SwapQuote> {
    let inner = unwrap_envelope(v);

    let route_id = str_at(inner, &["route_id", "quote_id", "id"])
        .map(str::to_string)
        .unwrap_or_else(|| format!("q_{}", intent.id));

    let expected_out_symbol = str_at(inner, &["expected_out_symbol", "out_symbol", "to_symbol"])
        .map(str::to_string)
        .unwrap_or_else(|| intent.to_symbol.clone());

    let expected_out_amount =
        decimal_at(inner, &["expected_out_amount", "out_amount", "amount_out"])
            .unwrap_or(intent.amount_usd);

    let summary_v = inner.get("summary").unwrap_or(inner);
    let summary = QuoteSummary {
        expected_out_usd: decimal_at(summary_v, &["expected_out_usd", "out_usd", "value_usd"])
            .unwrap_or(expected_out_amount),
        price_impact_pct: decimal_or_zero(
            summary_v,
            &["price_impact_pct", "price_impact", "impact"],
        ),
        slippage_pct: decimal_or_zero(summary_v, &["slippage_pct", "slippage"]),
        liquidity_usd: decimal_or_zero(summary_v, &["liquidity_usd", "liquidity", "pool_usd"]),
    };

    Some(SwapQuote {
        route_id,
        expected_out_symbol,
        expected_out_amount,
        summary,
    })
}

/// Build a `TxReceipt` from a swap/registration response, defaulting status to
/// "submitted" and tolerating a missing block number.
pub fn tx_receipt(v: &Value) -> TxReceipt {
    let inner = unwrap_envelope(v);
    let tx_hash = str_at(inner, &["tx_hash", "hash", "transaction_hash", "txid"])
        .unwrap_or("")
        .to_string();
    let status = str_at(inner, &["status", "state"])
        .unwrap_or("submitted")
        .to_string();
    let block_number = inner
        .get("block_number")
        .or_else(|| inner.get("block"))
        .and_then(Value::as_u64);

    TxReceipt {
        tx_hash,
        status,
        block_number,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::OrderSide;
    use serde_json::json;

    fn intent() -> OrderIntent {
        OrderIntent::new(OrderSide::Buy, "USDT", "BNB", Decimal::from(100), "test")
    }

    #[test]
    fn wallet_address_reads_nested_and_bare() {
        assert_eq!(
            wallet_address(&json!({"address": "0xabc"})).as_deref(),
            Some("0xabc")
        );
        assert_eq!(
            wallet_address(&json!({"result": {"wallet": "0xdef"}})).as_deref(),
            Some("0xdef")
        );
        assert_eq!(wallet_address(&json!("0x123")).as_deref(), Some("0x123"));
    }

    #[test]
    fn portfolio_handles_array_and_object() {
        let p = portfolio(&json!({"balances": [
            {"symbol": "USDT", "amount": "10000", "value_usd": 10000}
        ]}));
        assert_eq!(p.balances.len(), 1);
        assert_eq!(p.balances[0].symbol, "USDT");
        assert_eq!(p.balances[0].amount, Decimal::from(10000));

        let p2 = portfolio(&json!([{"asset": "BNB", "balance": "2.5"}]));
        assert_eq!(p2.balances.len(), 1);
        assert_eq!(p2.balances[0].symbol, "BNB");
    }

    #[test]
    fn portfolio_empty_on_garbage() {
        assert!(portfolio(&json!({"unexpected": true})).balances.is_empty());
    }

    #[test]
    fn swap_quote_falls_back_to_intent() {
        let q = swap_quote(&json!({}), &intent()).expect("quote");
        assert_eq!(q.expected_out_symbol, "BNB");
        assert_eq!(q.expected_out_amount, Decimal::from(100));
    }

    #[test]
    fn swap_quote_reads_summary() {
        let q = swap_quote(
            &json!({
                "route_id": "r1",
                "expected_out_amount": "99.5",
                "summary": {"price_impact_pct": "0.1", "slippage_pct": 0.05}
            }),
            &intent(),
        )
        .expect("quote");
        assert_eq!(q.route_id, "r1");
        assert_eq!(
            q.summary.price_impact_pct,
            Decimal::from_str("0.1").unwrap()
        );
    }

    #[test]
    fn tx_receipt_defaults() {
        let r = tx_receipt(&json!({"tx_hash": "0xfeed"}));
        assert_eq!(r.tx_hash, "0xfeed");
        assert_eq!(r.status, "submitted");
        assert!(r.block_number.is_none());

        let r2 =
            tx_receipt(&json!({"result": {"hash": "0xbeef", "status": "confirmed", "block": 42}}));
        assert_eq!(r2.tx_hash, "0xbeef");
        assert_eq!(r2.status, "confirmed");
        assert_eq!(r2.block_number, Some(42));
    }
}
