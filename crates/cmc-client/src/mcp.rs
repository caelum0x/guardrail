//! MCP (Model Context Protocol) implementation of [`CmcDataSource`].
//!
//! This talks JSON-RPC 2.0 over HTTP to a CoinMarketCap "AI Agent Hub" MCP
//! endpoint. Each [`CmcDataSource`] method maps to an MCP tool call (e.g.
//! `cmc_latest_quotes`, `cmc_fear_and_greed`). Parsing is intentionally
//! defensive: MCP tool results are loosely shaped, so we navigate
//! `serde_json::Value` and fall back to sane defaults rather than failing a
//! whole snapshot on one missing field — mirroring the REST client.

use crate::error::CmcError;
use crate::models::*;
use crate::CmcDataSource;
use async_trait::async_trait;
use common::time::now_ms;
use common::Asset;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::str::FromStr;
use std::time::Duration;

/// Configuration for the CMC MCP endpoint.
#[derive(Debug, Clone)]
pub struct CmcMcpConfig {
    pub url: String,
}

/// JSON-RPC 2.0 client for a CoinMarketCap MCP endpoint.
pub struct CmcMcpClient {
    url: String,
    http: reqwest::Client,
    api_key: Option<String>,
}

impl CmcMcpClient {
    /// Build a client pointed at an MCP endpoint URL.
    pub fn new(url: impl Into<String>, timeout_ms: u64) -> Result<Self, CmcError> {
        let url = url.into();
        if url.is_empty() {
            return Err(CmcError::Http("missing MCP endpoint url".into()));
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()?;
        Ok(CmcMcpClient {
            url,
            http,
            api_key: None,
        })
    }

    /// Attach the CMC MCP API key, sent as `X-CMC-MCP-API-KEY` on every call.
    /// Required by the official `mcp.coinmarketcap.com/mcp` endpoint; empty
    /// keys are ignored (e.g. when using the keyless x402 endpoint).
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        let key = key.into();
        self.api_key = (!key.trim().is_empty()).then_some(key);
        self
    }

    /// Build a client from a [`CmcMcpConfig`].
    pub fn from_config(cfg: &CmcMcpConfig, timeout_ms: u64) -> Result<Self, CmcError> {
        Self::new(cfg.url.clone(), timeout_ms)
    }

    /// Issue a JSON-RPC 2.0 request and return the `result` value.
    ///
    /// Maps transport errors and JSON-RPC `error` objects to [`CmcError`].
    pub async fn call(&self, method: &str, params: Value) -> Result<Value, CmcError> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let mut req = self
            .http
            .post(&self.url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json");
        if let Some(key) = &self.api_key {
            req = req.header(crate::endpoints::MCP_API_KEY_HEADER, key);
        }
        let resp = req.json(&body).send().await?;

        let status = resp.status().as_u16();
        if status == 429 {
            return Err(CmcError::RateLimited(2));
        }

        let envelope: Value = resp.json().await?;

        if let Some(err) = envelope.get("error") {
            if !err.is_null() {
                let msg = err
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown JSON-RPC error");
                return Err(CmcError::Http(format!("MCP error: {msg}")));
            }
        }

        envelope
            .get("result")
            .cloned()
            .ok_or_else(|| CmcError::Decode("MCP response missing result".into()))
    }

    /// Call an MCP tool by name with arguments and unwrap the tool payload.
    ///
    /// MCP tool calls go through `tools/call` with `{ name, arguments }`. The
    /// payload may surface directly as `result`, or be nested under
    /// `structuredContent`, or embedded as JSON text in `content[].text`.
    async fn tool(&self, tool: &str, arguments: Value) -> Result<Value, CmcError> {
        let result = self
            .call(
                "tools/call",
                json!({ "name": tool, "arguments": arguments }),
            )
            .await?;
        Ok(unwrap_tool_payload(result))
    }
}

/// Extract the meaningful payload from an MCP `tools/call` result, tolerating
/// the several shapes servers use to wrap tool output.
fn unwrap_tool_payload(result: Value) -> Value {
    // Preferred: structured content alongside the human-readable content.
    if let Some(structured) = result.get("structuredContent") {
        if !structured.is_null() {
            return structured.clone();
        }
    }
    // Common: an array of content blocks; the first text block holds JSON.
    if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
        for block in content {
            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                    return parsed;
                }
            }
        }
    }
    // Fallback: the result *is* the payload.
    result
}

/// Coerce a JSON value into a [`Decimal`], accepting numbers or strings.
fn dec(v: &Value) -> Option<Decimal> {
    match v {
        Value::Number(n) => Decimal::from_str(&n.to_string()).ok(),
        Value::String(s) => Decimal::from_str(s).ok(),
        _ => None,
    }
}

/// Navigate to the `data` node if present, otherwise treat the value as data.
fn data_node(v: &Value) -> &Value {
    v.get("data").unwrap_or(v)
}

#[async_trait]
impl CmcDataSource for CmcMcpClient {
    async fn latest_quotes(&self, assets: &[Asset]) -> Result<Vec<CmcQuote>, CmcError> {
        if assets.is_empty() {
            return Ok(vec![]);
        }
        let ids = assets
            .iter()
            .map(|a| a.cmc_id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let payload = self
            .tool("cmc_latest_quotes", json!({ "id": ids, "convert": "USD" }))
            .await?;
        let data = data_node(&payload);

        let mut out = Vec::with_capacity(assets.len());
        for asset in assets {
            // `data` may be keyed by id, by symbol, or be a flat array.
            let entry = data
                .get(asset.cmc_id.to_string())
                .or_else(|| data.get(&asset.symbol));
            let obj = match entry {
                Some(Value::Array(arr)) => arr.first().cloned(),
                Some(v) => Some(v.clone()),
                None => data.as_array().and_then(|arr| {
                    arr.iter()
                        .find(|q| q.get("id").and_then(|v| v.as_u64()) == Some(asset.cmc_id))
                        .cloned()
                }),
            };
            let Some(obj) = obj else { continue };
            let usd = obj.pointer("/quote/USD");
            out.push(CmcQuote {
                cmc_id: asset.cmc_id,
                symbol: asset.symbol.clone(),
                price_usd: usd.and_then(|u| dec(&u["price"])).unwrap_or_default(),
                volume_24h_usd: usd.and_then(|u| dec(&u["volume_24h"])).unwrap_or_default(),
                market_cap_usd: usd.and_then(|u| dec(&u["market_cap"])),
                percent_change_1h: usd.and_then(|u| dec(&u["percent_change_1h"])),
                percent_change_24h: usd.and_then(|u| dec(&u["percent_change_24h"])),
                last_updated_ms: now_ms(),
            });
        }
        Ok(out)
    }

    async fn ohlcv(&self, asset: &Asset, interval: Interval) -> Result<Vec<Candle>, CmcError> {
        let payload = self
            .tool(
                "cmc_ohlcv",
                json!({
                    "id": asset.cmc_id.to_string(),
                    "interval": interval.as_str(),
                    "convert": "USD",
                }),
            )
            .await?;
        // Quotes may live at /data/quotes or directly at /quotes.
        let quotes = payload
            .pointer("/data/quotes")
            .or_else(|| payload.pointer("/quotes"))
            .and_then(|q| q.as_array());
        let Some(quotes) = quotes else {
            return Err(CmcError::NotFound(format!("ohlcv {}", asset.symbol)));
        };
        let candles = quotes
            .iter()
            .filter_map(|q| {
                let usd = q.pointer("/quote/USD").or(Some(q))?;
                Some(Candle {
                    open_time_ms: now_ms(),
                    open: dec(&usd["open"]).unwrap_or_default(),
                    high: dec(&usd["high"]).unwrap_or_default(),
                    low: dec(&usd["low"]).unwrap_or_default(),
                    close: dec(&usd["close"]).unwrap_or_default(),
                    volume: dec(&usd["volume"]).unwrap_or_default(),
                })
            })
            .collect();
        Ok(candles)
    }

    async fn fear_greed(&self) -> Result<FearGreedSnapshot, CmcError> {
        let payload = self.tool("cmc_fear_and_greed", json!({})).await?;
        let data = data_node(&payload);
        Ok(FearGreedSnapshot {
            value: data.get("value").and_then(|v| v.as_u64()).unwrap_or(50) as u32,
            classification: data
                .get("value_classification")
                .or_else(|| data.get("classification"))
                .and_then(|v| v.as_str())
                .unwrap_or("Neutral")
                .to_string(),
            updated_ms: now_ms(),
        })
    }

    async fn dex_liquidity(&self, asset: &Asset) -> Result<DexLiquidity, CmcError> {
        let payload = self
            .tool("cmc_dex_liquidity", json!({ "symbol": asset.symbol }))
            .await?;
        // Aggregate liquidity may be at /data/0/liquidity or /liquidity.
        let liq = payload
            .pointer("/data/0/liquidity")
            .or_else(|| payload.pointer("/data/liquidity"))
            .or_else(|| payload.pointer("/liquidity"))
            .and_then(dec)
            .unwrap_or_default();
        Ok(DexLiquidity {
            symbol: asset.symbol.clone(),
            total_liquidity_usd: liq,
            top_pair_liquidity_usd: liq,
            pair_count: 1,
        })
    }

    async fn token_security(&self, asset: &Asset) -> Result<TokenSecurity, CmcError> {
        let payload = self
            .tool(
                "cmc_token_security",
                json!({
                    "symbol": asset.symbol,
                    "address": asset.contract_address,
                    "chain_id": asset.chain_id,
                }),
            )
            .await;
        // Security data may be unavailable on some endpoints; default to safe.
        let Ok(payload) = payload else {
            return Ok(TokenSecurity {
                symbol: asset.symbol.clone(),
                flags: vec![],
                safety_score: 80,
            });
        };
        let data = data_node(&payload);
        let flags = data
            .get("flags")
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let safety_score = data
            .get("safety_score")
            .or_else(|| data.get("score"))
            .and_then(|v| v.as_u64())
            .unwrap_or(80) as u32;
        Ok(TokenSecurity {
            symbol: asset.symbol.clone(),
            flags,
            safety_score,
        })
    }

    async fn trending(&self) -> Result<Vec<TrendingToken>, CmcError> {
        let payload = self.tool("cmc_trending", json!({})).await?;
        let arr = data_node(&payload).as_array().cloned();
        let mut out = vec![];
        if let Some(arr) = arr {
            for (i, t) in arr.iter().enumerate() {
                out.push(TrendingToken {
                    cmc_id: t.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                    symbol: t
                        .get("symbol")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    rank: t
                        .get("rank")
                        .and_then(|v| v.as_u64())
                        .unwrap_or((i + 1) as u64) as u32,
                });
            }
        }
        Ok(out)
    }

    async fn global_market(&self) -> Result<GlobalMarket, CmcError> {
        let payload = self.tool("cmc_global_market", json!({})).await?;
        let usd = payload
            .pointer("/data/quote/USD")
            .or_else(|| payload.pointer("/quote/USD"));
        Ok(GlobalMarket {
            total_market_cap_usd: usd
                .and_then(|u| dec(&u["total_market_cap"]))
                .unwrap_or_default(),
            btc_dominance_pct: payload
                .pointer("/data/btc_dominance")
                .or_else(|| payload.pointer("/btc_dominance"))
                .and_then(dec)
                .unwrap_or_default(),
            updated_ms: now_ms(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_empty_url() {
        let err = CmcMcpClient::new("", 1000);
        assert!(err.is_err());
    }

    #[test]
    fn new_accepts_url() {
        let ok = CmcMcpClient::new("https://mcp.coinmarketcap.com", 1000);
        assert!(ok.is_ok());
    }

    #[test]
    fn unwrap_prefers_structured_content() {
        let result = json!({
            "structuredContent": { "data": { "value": 42 } },
            "content": [{ "type": "text", "text": "ignored" }],
        });
        let payload = unwrap_tool_payload(result);
        assert_eq!(
            payload.pointer("/data/value").and_then(|v| v.as_u64()),
            Some(42)
        );
    }

    #[test]
    fn unwrap_parses_text_content_json() {
        let result = json!({
            "content": [{ "type": "text", "text": "{\"data\":{\"value\":7}}" }],
        });
        let payload = unwrap_tool_payload(result);
        assert_eq!(
            payload.pointer("/data/value").and_then(|v| v.as_u64()),
            Some(7)
        );
    }

    #[test]
    fn unwrap_falls_back_to_raw_result() {
        let result = json!({ "data": { "value": 9 } });
        let payload = unwrap_tool_payload(result);
        assert_eq!(
            payload.pointer("/data/value").and_then(|v| v.as_u64()),
            Some(9)
        );
    }

    #[test]
    fn data_node_handles_both_shapes() {
        let wrapped = json!({ "data": { "x": 1 } });
        assert_eq!(
            data_node(&wrapped).get("x").and_then(|v| v.as_u64()),
            Some(1)
        );
        let flat = json!({ "x": 1 });
        assert_eq!(data_node(&flat).get("x").and_then(|v| v.as_u64()), Some(1));
    }
}
