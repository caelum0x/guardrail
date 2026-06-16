//! Correlation endpoint: `GET /correlation`.
//!
//! Computes a pairwise Pearson correlation matrix over named return series
//! using the real `correlation` crate. Read-only and pure.
//!
//! `?series=` is a `;`-separated list of `name:v1,v2,v3,…` entries. Example:
//! `series=BTC:0.01,-0.02,0.03;ETH:0.012,-0.018,0.025`

use std::collections::BTreeMap;

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct CorrelationQuery {
    series: Option<String>,
}

const DEFAULT_SERIES: &str =
    "BTC:0.01,-0.02,0.03,-0.01,0.02;ETH:0.012,-0.018,0.028,-0.008,0.022;CAKE:-0.005,0.03,-0.02,0.01,-0.015";

fn parse_series(spec: &str) -> Result<BTreeMap<String, Vec<f64>>, String> {
    let mut out = BTreeMap::new();
    for (i, raw) in spec.split(';').filter(|s| !s.trim().is_empty()).enumerate() {
        let (name, values) = raw
            .split_once(':')
            .ok_or_else(|| format!("series {i}: expected 'name:v1,v2,…', got '{raw}'"))?;
        let parsed: Vec<f64> = values.split(',').filter_map(|v| v.trim().parse().ok()).collect();
        if parsed.len() < 2 {
            return Err(format!("series '{}' needs at least 2 values", name.trim()));
        }
        out.insert(name.trim().to_string(), parsed);
    }
    if out.len() < 2 {
        return Err("need at least 2 named series".to_string());
    }
    Ok(out)
}

pub async fn correlation(Query(q): Query<CorrelationQuery>) -> Json<Value> {
    let spec = q.series.unwrap_or_else(|| DEFAULT_SERIES.to_string());
    let series = match parse_series(&spec) {
        Ok(s) => s,
        Err(e) => return Json(json!({ "error": e, "format": "name:v1,v2,…;name2:… " })),
    };
    let matrix = correlation::correlation_matrix(&series);
    Json(json!({
        "spec": spec,
        "names": matrix.names,
        "matrix": matrix.matrix,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_series() {
        let s = parse_series("A:1,2,3;B:3,2,1").unwrap();
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn rejects_single_series() {
        assert!(parse_series("A:1,2,3").is_err());
        assert!(parse_series("A:1").is_err());
    }
}
