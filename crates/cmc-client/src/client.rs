//! REST implementation of [`CmcDataSource`] against the CMC Pro API.
//!
//! Parsing is intentionally defensive: CMC responses are large and nested, so
//! we navigate `serde_json::Value` and fall back to sane defaults rather than
//! failing a whole snapshot on one missing field.

use crate::endpoints;
use crate::error::CmcError;
use crate::models::*;
use crate::rate_limit::RateLimiter;
use crate::retry::with_retry;
use crate::x402;
use crate::CmcDataSource;
use async_trait::async_trait;
use common::time::now_ms;
use common::Asset;
use rust_decimal::Decimal;
use serde_json::Value;
use std::str::FromStr;
use std::time::Duration;

pub struct CmcRestClient {
    http: reqwest::Client,
    api_key: String,
    limiter: RateLimiter,
}

impl CmcRestClient {
    pub fn new(api_key: impl Into<String>, timeout_ms: u64) -> Result<Self, CmcError> {
        let key = api_key.into();
        if key.is_empty() {
            return Err(CmcError::MissingApiKey);
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()?;
        Ok(CmcRestClient {
            http,
            api_key: key,
            limiter: RateLimiter::per_minute(30),
        })
    }

    async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Value, CmcError> {
        self.limiter.acquire().await;
        let url = format!("{}{}", endpoints::BASE_URL, path);
        with_retry(3, || async {
            let resp = self
                .http
                .get(&url)
                .header("X-CMC_PRO_API_KEY", &self.api_key)
                .header("Accept", "application/json")
                .query(query)
                .send()
                .await?;
            let status = resp.status().as_u16();
            if status == 429 {
                return Err(CmcError::RateLimited(2));
            }
            // x402: endpoint demands payment. Build a payment payload from the
            // advertised terms and retry once with the X-PAYMENT header.
            if status == 402 {
                return self.pay_and_retry(&url, query, resp).await;
            }
            let json: Value = resp.json().await?;
            Ok(json)
        })
        .await
    }

    /// Handle an HTTP 402 by constructing an x402 payment payload from the
    /// response terms and replaying the request with the `X-PAYMENT` header.
    /// The payer address and signature come from the environment so this client
    /// never holds keys — TWAK signs the authorization out of band.
    async fn pay_and_retry(
        &self,
        url: &str,
        query: &[(&str, String)],
        resp: reqwest::Response,
    ) -> Result<Value, CmcError> {
        if !x402::is_enabled() {
            return Err(CmcError::Http(
                "402 payment required but x402 is disabled".into(),
            ));
        }
        let terms: x402::PaymentRequirements = resp
            .json()
            .await
            .map_err(|e| CmcError::Decode(format!("invalid x402 terms: {e}")))?;

        let from = std::env::var("CMC_X402_FROM").unwrap_or_default();
        let signature = std::env::var("CMC_X402_SIGNATURE").unwrap_or_default();
        let payment =
            x402::PaymentPayload::from_requirements(&terms, from).with_signature(signature);

        let resp = self
            .http
            .get(url)
            .header("X-CMC_PRO_API_KEY", &self.api_key)
            .header("Accept", "application/json")
            .header(x402::PAYMENT_HEADER, payment.header_value())
            .query(query)
            .send()
            .await?;
        let json: Value = resp.json().await?;
        Ok(json)
    }
}

fn dec(v: &Value) -> Option<Decimal> {
    match v {
        Value::Number(n) => Decimal::from_str(&n.to_string()).ok(),
        Value::String(s) => Decimal::from_str(s).ok(),
        _ => None,
    }
}

/// Parse the `data` payload of a CMC `quotes/latest` response into normalized
/// [`CmcQuote`] rows, one per requested asset that has valid data.
///
/// `data` may be keyed by `cmc_id` with either an object or a single-element
/// array value. A missing or non-positive (`<= 0`) price is treated as invalid
/// data — the asset is skipped this cycle (with a `tracing::warn`) rather than
/// emitting a misleading `$0` quote into the strategy/risk pipeline.
///
/// Pure and side-effect-free apart from logging, so the HTTP path and unit tests
/// share the exact same parsing behavior.
fn parse_quotes(data: &Value, assets: &[Asset]) -> Vec<CmcQuote> {
    let mut out = Vec::new();
    for asset in assets {
        let entry = data.get(asset.cmc_id.to_string());
        // `data` may be keyed by id with an object or array value.
        let obj = match entry {
            Some(Value::Array(arr)) => arr.first().cloned(),
            Some(v) => Some(v.clone()),
            None => None,
        };
        let Some(obj) = obj else { continue };
        let usd = obj.pointer("/quote/USD");
        // A missing or non-positive price is invalid data, not a $0 signal.
        // Skip the asset this cycle (and warn) rather than feeding the
        // strategy/risk engine a zero price.
        let price_usd = match usd.and_then(|u| dec(&u["price"])) {
            Some(price) if price > Decimal::ZERO => price,
            _ => {
                tracing::warn!(
                    symbol = %asset.symbol,
                    cmc_id = asset.cmc_id,
                    "CMC quote missing or non-positive price; skipping asset this cycle"
                );
                continue;
            }
        };
        out.push(CmcQuote {
            cmc_id: asset.cmc_id,
            symbol: asset.symbol.clone(),
            price_usd,
            volume_24h_usd: usd.and_then(|u| dec(&u["volume_24h"])).unwrap_or_default(),
            market_cap_usd: usd.and_then(|u| dec(&u["market_cap"])),
            percent_change_1h: usd.and_then(|u| dec(&u["percent_change_1h"])),
            percent_change_24h: usd.and_then(|u| dec(&u["percent_change_24h"])),
            last_updated_ms: now_ms(),
        });
    }
    out
}

#[async_trait]
impl CmcDataSource for CmcRestClient {
    async fn latest_quotes(&self, assets: &[Asset]) -> Result<Vec<CmcQuote>, CmcError> {
        if assets.is_empty() {
            return Ok(vec![]);
        }
        let ids = assets
            .iter()
            .map(|a| a.cmc_id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let json = self
            .get(
                endpoints::LATEST_QUOTES,
                &[("id", ids), ("convert", "USD".into())],
            )
            .await?;

        let data = json
            .get("data")
            .ok_or_else(|| CmcError::Decode("missing data".into()))?;

        Ok(parse_quotes(data, assets))
    }

    async fn ohlcv(&self, asset: &Asset, interval: Interval) -> Result<Vec<Candle>, CmcError> {
        let json = self
            .get(
                endpoints::OHLCV,
                &[
                    ("id", asset.cmc_id.to_string()),
                    ("interval", interval.as_str().to_string()),
                    ("convert", "USD".into()),
                ],
            )
            .await?;
        let quotes = json.pointer("/data/quotes").and_then(|q| q.as_array());
        let Some(quotes) = quotes else {
            return Err(CmcError::NotFound(format!("ohlcv {}", asset.symbol)));
        };
        let candles = quotes
            .iter()
            .filter_map(|q| {
                let usd = q.pointer("/quote/USD")?;
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
        let json = self.get(endpoints::FEAR_GREED, &[]).await?;
        let data = json.get("data").unwrap_or(&Value::Null);
        Ok(FearGreedSnapshot {
            value: data.get("value").and_then(|v| v.as_u64()).unwrap_or(50) as u32,
            classification: data
                .get("value_classification")
                .and_then(|v| v.as_str())
                .unwrap_or("Neutral")
                .to_string(),
            updated_ms: now_ms(),
        })
    }

    async fn dex_liquidity(&self, asset: &Asset) -> Result<DexLiquidity, CmcError> {
        // The DEX endpoint shape varies by plan; we surface an aggregate.
        let json = self
            .get(
                endpoints::DEX_LIQUIDITY,
                &[("symbol", asset.symbol.clone())],
            )
            .await?;
        let liq = json
            .pointer("/data/0/liquidity")
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
        // Security data is not in the base API; default to safe when unknown.
        Ok(TokenSecurity {
            symbol: asset.symbol.clone(),
            flags: vec![],
            safety_score: 80,
        })
    }

    async fn trending(&self) -> Result<Vec<TrendingToken>, CmcError> {
        let json = self.get(endpoints::TRENDING, &[]).await?;
        let arr = json.pointer("/data").and_then(|v| v.as_array());
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
                    rank: (i + 1) as u32,
                });
            }
        }
        Ok(out)
    }

    async fn global_market(&self) -> Result<GlobalMarket, CmcError> {
        let json = self.get(endpoints::GLOBAL_METRICS, &[]).await?;
        let usd = json.pointer("/data/quote/USD");
        Ok(GlobalMarket {
            total_market_cap_usd: usd
                .and_then(|u| dec(&u["total_market_cap"]))
                .unwrap_or_default(),
            btc_dominance_pct: json
                .pointer("/data/btc_dominance")
                .and_then(dec)
                .unwrap_or_default(),
            updated_ms: now_ms(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::parse_quotes;
    use common::{Asset, AssetCategory};
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::str::FromStr;

    fn asset(symbol: &str, cmc_id: u64) -> Asset {
        Asset {
            symbol: symbol.to_string(),
            cmc_id,
            chain_id: 56,
            contract_address: "0x0".to_string(),
            decimals: 18,
            category: AssetCategory::Core,
        }
    }

    /// Build a `data` payload keyed by cmc_id, where each (id, price) pair is
    /// rendered as the real CMC `quotes/latest` per-asset shape. A `None` price
    /// omits the `price` field entirely (missing-field case).
    fn data_with(entries: &[(u64, Option<f64>)]) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        for (id, price) in entries {
            let usd = match price {
                Some(p) => json!({
                    "price": p,
                    "volume_24h": 1000.0,
                    "market_cap": 5000.0,
                    "percent_change_1h": 0.5,
                    "percent_change_24h": -1.2,
                }),
                None => json!({
                    "volume_24h": 1000.0,
                    "market_cap": 5000.0,
                }),
            };
            map.insert(
                id.to_string(),
                json!({ "quote": { "USD": usd } }),
            );
        }
        serde_json::Value::Object(map)
    }

    #[test]
    fn valid_positive_price_is_included() {
        let assets = vec![asset("BTC", 1)];
        let data = data_with(&[(1, Some(42_000.5))]);

        let quotes = parse_quotes(&data, &assets);

        assert_eq!(quotes.len(), 1, "asset with a positive price must be kept");
        let q = &quotes[0];
        assert_eq!(q.cmc_id, 1);
        assert_eq!(q.symbol, "BTC");
        assert_eq!(q.price_usd, Decimal::from_str("42000.5").unwrap());
        assert_eq!(q.volume_24h_usd, Decimal::from_str("1000").unwrap());
    }

    #[test]
    fn zero_price_is_skipped() {
        let assets = vec![asset("BTC", 1)];
        let data = data_with(&[(1, Some(0.0))]);

        let quotes = parse_quotes(&data, &assets);

        assert!(
            quotes.is_empty(),
            "a zero price is invalid data, not a $0 signal — asset must be skipped"
        );
    }

    #[test]
    fn negative_price_is_skipped() {
        let assets = vec![asset("BTC", 1)];
        let data = data_with(&[(1, Some(-5.0))]);

        let quotes = parse_quotes(&data, &assets);

        assert!(quotes.is_empty(), "a non-positive price must be skipped");
    }

    #[test]
    fn missing_price_field_is_skipped() {
        let assets = vec![asset("BTC", 1)];
        let data = data_with(&[(1, None)]);

        let quotes = parse_quotes(&data, &assets);

        assert!(
            quotes.is_empty(),
            "a quote with no price field must be skipped, not defaulted to $0"
        );
    }

    #[test]
    fn mixed_assets_skip_only_the_invalid_ones() {
        let assets = vec![asset("BTC", 1), asset("ETH", 2), asset("DOGE", 3)];
        // BTC valid, ETH zero (skip), DOGE missing price (skip).
        let data = data_with(&[(1, Some(42_000.0)), (2, Some(0.0)), (3, None)]);

        let quotes = parse_quotes(&data, &assets);

        assert_eq!(quotes.len(), 1, "only the valid asset should survive");
        assert_eq!(quotes[0].symbol, "BTC");
    }

    #[test]
    fn array_keyed_entry_is_parsed() {
        // CMC sometimes returns the per-id value as a single-element array.
        let assets = vec![asset("BTC", 1)];
        let data = json!({
            "1": [ { "quote": { "USD": { "price": 100.0, "volume_24h": 1.0 } } } ]
        });

        let quotes = parse_quotes(&data, &assets);

        assert_eq!(quotes.len(), 1);
        assert_eq!(quotes[0].price_usd, Decimal::from_str("100").unwrap());
    }
}
