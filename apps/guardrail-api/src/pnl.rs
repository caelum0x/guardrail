//! PnL attribution endpoint: `GET /pnl`.
//!
//! Computes average-cost realized/unrealized PnL per symbol from a fill spec,
//! using the real `pnl-attribution` crate. Read-only and pure.
//!
//! `?fills=` is a `;`-separated list of `symbol,side,qty,price[,fee]` entries
//! (`side` is `buy`/`sell`). `?marks=` is a `,`-separated list of `SYMBOL:price`
//! used to value open positions. Example:
//! `fills=CAKE,buy,10,2;CAKE,sell,4,3&marks=CAKE:3`

use std::collections::BTreeMap;
use std::str::FromStr;

use axum::extract::Query;
use axum::Json;
use pnl_attribution::{Attributor, Fill, Side};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct PnlQuery {
    fills: Option<String>,
    marks: Option<String>,
}

const DEFAULT_FILLS: &str = "CAKE,buy,10,2;CAKE,sell,4,3;WBNB,buy,5,600";

fn parse_fills(spec: &str) -> Result<Vec<Fill>, String> {
    let mut out = Vec::new();
    for (i, raw) in spec.split(';').filter(|s| !s.trim().is_empty()).enumerate() {
        let p: Vec<&str> = raw.split(',').map(str::trim).collect();
        if p.len() < 4 {
            return Err(format!("fill {i}: expected 'symbol,side,qty,price[,fee]'"));
        }
        let side = match p[1].to_lowercase().as_str() {
            "buy" | "b" => Side::Buy,
            "sell" | "s" => Side::Sell,
            other => return Err(format!("fill {i}: bad side '{other}'")),
        };
        let qty = Decimal::from_str(p[2]).map_err(|_| format!("fill {i}: bad qty"))?;
        let price = Decimal::from_str(p[3]).map_err(|_| format!("fill {i}: bad price"))?;
        let fee = p.get(4).and_then(|s| Decimal::from_str(s).ok()).unwrap_or(Decimal::ZERO);
        out.push(Fill::new(p[0], side, qty, price, fee));
    }
    Ok(out)
}

fn parse_marks(spec: &str) -> BTreeMap<String, Decimal> {
    let mut out = BTreeMap::new();
    for pair in spec.split(',').filter(|s| !s.trim().is_empty()) {
        if let Some((sym, price)) = pair.split_once(':') {
            if let Ok(p) = Decimal::from_str(price.trim()) {
                out.insert(sym.trim().to_string(), p);
            }
        }
    }
    out
}

pub async fn pnl(Query(q): Query<PnlQuery>) -> Json<Value> {
    let spec = q.fills.unwrap_or_else(|| DEFAULT_FILLS.to_string());
    let fills = match parse_fills(&spec) {
        Ok(f) => f,
        Err(e) => return Json(json!({ "error": e })),
    };
    let marks = q.marks.as_deref().map(parse_marks).unwrap_or_default();

    let mut attr = Attributor::new();
    attr.apply_all(&fills);
    let report = attr.report(&marks);

    Json(json!({
        "fills": spec,
        "marks": marks.iter().map(|(k, v)| (k.clone(), v.to_string())).collect::<BTreeMap<_, _>>(),
        "report": report,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fills_and_marks() {
        let fills = parse_fills("CAKE,buy,10,2;CAKE,sell,4,3").unwrap();
        assert_eq!(fills.len(), 2);
        let marks = parse_marks("CAKE:3,WBNB:600");
        assert_eq!(marks.len(), 2);
    }

    #[test]
    fn rejects_bad_fill() {
        assert!(parse_fills("CAKE,buy,10").is_err());
        assert!(parse_fills("CAKE,hodl,10,2").is_err());
    }
}
