//! Portfolio-optimizer endpoint.
//!
//! Visualizes the alternative allocation methods exposed by the
//! `portfolio-optimizer` crate over a small example basket. Computes weights
//! for all four methods (equal weight, score proportional, inverse volatility,
//! risk parity) and returns them as a unit-budget allocation. Read-only and
//! side-effect free.

use axum::extract::Query;
use axum::Json;
use portfolio_optimizer::{equal_weight, inverse_volatility, risk_parity_lite, score_proportional};
use serde::Deserialize;
use serde_json::{json, Value};

/// Weights are computed against a unit budget so they read as fractions of the
/// portfolio (0..1) regardless of the basket.
const BUDGET: f64 = 1.0;

#[derive(Debug, Deserialize)]
pub struct OptimizeParams {
    /// Comma-separated symbols (default CAKE,WBNB,BTCB).
    pub symbols: Option<String>,
    /// Comma-separated scores aligned to `symbols`.
    pub scores: Option<String>,
    /// Comma-separated volatilities aligned to `symbols`.
    pub vols: Option<String>,
}

/// Parse a comma-separated list of `f64`, dropping empty/invalid entries.
fn parse_f64_list(raw: &str) -> Vec<f64> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .collect()
}

/// Parse a comma-separated list of symbols, dropping empty entries.
fn parse_symbols(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

pub async fn optimize(Query(params): Query<OptimizeParams>) -> Json<Value> {
    Json(build(&params))
}

/// Build the allocation comparison for the (possibly defaulted) basket.
fn build(params: &OptimizeParams) -> Value {
    let (symbols, scores, vols) = basket(params);

    json!({
        "symbols": symbols,
        "scores": scores,
        "vols": vols,
        "methods": {
            "equal_weight": equal_weight(scores.len(), BUDGET),
            "score_proportional": score_proportional(&scores, BUDGET),
            "inverse_volatility": inverse_volatility(&vols, BUDGET),
            "risk_parity": risk_parity_lite(&vols, BUDGET),
        },
    })
}

/// Resolve the basket from query params, falling back to a fixed example when
/// the inputs are absent or inconsistent (mismatched lengths / empty lists).
fn basket(params: &OptimizeParams) -> (Vec<String>, Vec<f64>, Vec<f64>) {
    let symbols = params.symbols.as_deref().map(parse_symbols);
    let scores = params.scores.as_deref().map(parse_f64_list);
    let vols = params.vols.as_deref().map(parse_f64_list);

    if let (Some(symbols), Some(scores), Some(vols)) = (symbols, scores, vols) {
        let n = symbols.len();
        if n > 0 && scores.len() == n && vols.len() == n {
            return (symbols, scores, vols);
        }
    }

    (
        vec!["CAKE".to_string(), "WBNB".to_string(), "BTCB".to_string()],
        vec![0.8, 0.6, 0.5],
        vec![3.0, 2.0, 5.0],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_params() -> OptimizeParams {
        OptimizeParams {
            symbols: None,
            scores: None,
            vols: None,
        }
    }

    #[test]
    fn build_uses_example_basket_by_default() {
        let value = build(&default_params());
        assert_eq!(value["symbols"][0], "CAKE");
        let methods = &value["methods"];
        for key in [
            "equal_weight",
            "score_proportional",
            "inverse_volatility",
            "risk_parity",
        ] {
            let weights = methods[key]
                .as_array()
                .unwrap_or_else(|| panic!("{key} should be an array"));
            assert_eq!(weights.len(), 3);
            let sum: f64 = weights.iter().filter_map(|w| w.as_f64()).sum();
            assert!((sum - 1.0).abs() < 1e-9, "{key} should sum to budget");
        }
    }

    #[test]
    fn build_accepts_query_basket() {
        let params = OptimizeParams {
            symbols: Some("AAA,BBB".to_string()),
            scores: Some("1.0,3.0".to_string()),
            vols: Some("2.0,4.0".to_string()),
        };
        let value = build(&params);
        assert_eq!(value["symbols"].as_array().map(Vec::len), Some(2));
        assert_eq!(value["scores"][1], 3.0);
    }

    #[test]
    fn build_falls_back_on_mismatched_lengths() {
        let params = OptimizeParams {
            symbols: Some("AAA,BBB".to_string()),
            scores: Some("1.0".to_string()),
            vols: Some("2.0,4.0".to_string()),
        };
        let value = build(&params);
        assert_eq!(value["symbols"][0], "CAKE");
    }
}
