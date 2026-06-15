//! Trending-tokens endpoint.
//!
//! Surfaces the trending tokens reported by the CMC data source. Uses the
//! deterministic [`MockCmcClient`] so the endpoint is reproducible and free of
//! network access. Read-only and side-effect free.

use axum::Json;
use cmc_client::{CmcDataSource, MockCmcClient};
use serde_json::{json, Value};

pub async fn trending() -> Json<Value> {
    let client = MockCmcClient::new();
    match client.trending().await {
        Ok(tokens) => {
            let rows: Vec<Value> = tokens
                .iter()
                .map(|t| {
                    json!({
                        "rank": t.rank,
                        "symbol": t.symbol,
                        "cmc_id": t.cmc_id,
                    })
                })
                .collect();
            Json(json!({ "tokens": rows }))
        }
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_trending_tokens() {
        let Json(value) = trending().await;
        let tokens = value["tokens"]
            .as_array()
            .expect("tokens should be an array");
        assert!(!tokens.is_empty());
        let first = &tokens[0];
        assert!(first["rank"].is_number());
        assert!(first["symbol"].is_string());
        assert!(first["cmc_id"].is_number());
    }
}
