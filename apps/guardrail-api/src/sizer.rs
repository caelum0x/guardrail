//! Position-sizing endpoint: `GET /sizer`.
//!
//! Computes a position size using the real `position-sizer` crate. Read-only,
//! pure. Select the method with `?method=`:
//! - `fixed_fractional` — `equity, risk_fraction, entry_price, risk_per_unit`
//! - `vol_target`       — `capital, target_vol, asset_vol, max_leverage`
//! - `kelly`            — `win_prob, odds, fraction, cap`

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use position_sizer::{
    fixed_fractional, kelly_fraction, vol_target, FixedFractionalInput, KellyInput, VolTargetInput,
};

#[derive(Debug, Deserialize)]
pub struct SizerQuery {
    method: String,
    // fixed_fractional
    equity: Option<f64>,
    risk_fraction: Option<f64>,
    entry_price: Option<f64>,
    risk_per_unit: Option<f64>,
    // vol_target
    capital: Option<f64>,
    target_vol: Option<f64>,
    asset_vol: Option<f64>,
    max_leverage: Option<f64>,
    // kelly
    win_prob: Option<f64>,
    odds: Option<f64>,
    fraction: Option<f64>,
    cap: Option<f64>,
}

fn err(msg: impl Into<String>) -> Json<Value> {
    Json(json!({ "error": msg.into() }))
}

pub async fn sizer(Query(q): Query<SizerQuery>) -> Json<Value> {
    match q.method.as_str() {
        "fixed_fractional" => {
            let input = FixedFractionalInput {
                equity: q.equity.unwrap_or(10_000.0),
                risk_fraction: q.risk_fraction.unwrap_or(0.02),
                entry_price: q.entry_price.unwrap_or(100.0),
                risk_per_unit: q.risk_per_unit.unwrap_or(5.0),
            };
            match fixed_fractional(input) {
                Ok(out) => Json(json!({ "method": "fixed_fractional", "input": input, "output": out })),
                Err(e) => err(e.to_string()),
            }
        }
        "vol_target" => {
            let input = VolTargetInput {
                capital: q.capital.unwrap_or(10_000.0),
                target_vol: q.target_vol.unwrap_or(0.15),
                asset_vol: q.asset_vol.unwrap_or(0.6),
                max_leverage: q.max_leverage.unwrap_or(3.0),
            };
            match vol_target(input) {
                Ok(out) => Json(json!({ "method": "vol_target", "input": input, "output": out })),
                Err(e) => err(e.to_string()),
            }
        }
        "kelly" => {
            let input = KellyInput {
                win_prob: q.win_prob.unwrap_or(0.55),
                odds: q.odds.unwrap_or(1.0),
                fraction: q.fraction.unwrap_or(0.5),
                cap: q.cap.unwrap_or(0.25),
            };
            match kelly_fraction(input) {
                Ok(out) => Json(json!({ "method": "kelly", "input": input, "output": out })),
                Err(e) => err(e.to_string()),
            }
        }
        other => Json(json!({
            "error": format!("unknown method '{other}'"),
            "methods": ["fixed_fractional", "vol_target", "kelly"],
        })),
    }
}
