//! Deterministic mock data source for paper trading and tests.
//!
//! Values are derived from the asset symbol and a monotonically advancing tick
//! so runs are reproducible and free of network access. The numbers are shaped
//! to exercise the strategy and risk paths (momentum, liquidity, sentiment).

use crate::error::CmcError;
use crate::models::*;
use crate::CmcDataSource;
use async_trait::async_trait;
use common::time::now_ms;
use common::Asset;
use rust_decimal::Decimal;
use std::sync::atomic::{AtomicI64, Ordering};

pub struct MockCmcClient {
    tick: AtomicI64,
    fear_greed: u32,
}

impl Default for MockCmcClient {
    fn default() -> Self {
        MockCmcClient {
            tick: AtomicI64::new(0),
            fear_greed: 58,
        }
    }
}

impl MockCmcClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_fear_greed(value: u32) -> Self {
        MockCmcClient {
            tick: AtomicI64::new(0),
            fear_greed: value,
        }
    }
}

/// Cheap deterministic hash of a symbol into a 0..1000 bucket.
fn seed(symbol: &str) -> i64 {
    symbol
        .bytes()
        .fold(0i64, |acc, b| (acc * 31 + b as i64) % 1000)
}

fn d(n: i64, scale: u32) -> Decimal {
    Decimal::new(n, scale)
}

#[async_trait]
impl CmcDataSource for MockCmcClient {
    async fn latest_quotes(&self, assets: &[Asset]) -> Result<Vec<CmcQuote>, CmcError> {
        let tick = self.tick.fetch_add(1, Ordering::Relaxed);
        let mut out = Vec::with_capacity(assets.len());
        for a in assets {
            let s = seed(&a.symbol);
            let is_stable = a.category.is_stable();
            // Stables hover at $1; others get a symbol-derived price.
            let price = if is_stable {
                Decimal::ONE
            } else {
                d(100 + s, 2) // e.g. 5.23 .. 11.99
            };
            // Constructive, symbol-differentiated tape: a few clear leaders so
            // the regime classifier sees breadth and the scorer separates names.
            // `lead` in 0..4 sets the strength; `tick` adds gentle variation.
            let lead = (s % 5) + 1; // 1..5
            let osc = (tick + s) % 3; // 0..2
            let r1h = d((lead * 80) + osc * 15, 2); // 0.95% .. 4.30%
            let r24h = d((lead * 250) + osc * 40, 2); // 2.90% .. 13.30%
            out.push(CmcQuote {
                cmc_id: a.cmc_id,
                symbol: a.symbol.clone(),
                price_usd: price,
                volume_24h_usd: d(8_000_000 + s * 20_000, 0),
                market_cap_usd: Some(d(50_000_000 + s * 100_000, 0)),
                percent_change_1h: Some(if is_stable { Decimal::ZERO } else { r1h }),
                percent_change_24h: Some(if is_stable { Decimal::ZERO } else { r24h }),
                last_updated_ms: now_ms(),
            });
        }
        Ok(out)
    }

    async fn ohlcv(&self, asset: &Asset, _interval: Interval) -> Result<Vec<Candle>, CmcError> {
        let s = seed(&asset.symbol);
        let base = 100 + s;
        let candles = (0..48)
            .map(|i| {
                let wobble = ((i + s) % 11) - 5;
                let close = d(base + wobble, 2);
                // ~1.5% intrabar band -> ~3% range, the strategy's sweet spot.
                let band = close * Decimal::new(15, 3);
                Candle {
                    open_time_ms: now_ms() - (48 - i) * 3_600_000,
                    open: close,
                    high: close + band,
                    low: close - band,
                    close,
                    volume: d(8_000_000 + s * 20_000, 0),
                }
            })
            .collect();
        Ok(candles)
    }

    async fn fear_greed(&self) -> Result<FearGreedSnapshot, CmcError> {
        let classification = match self.fear_greed {
            0..=24 => "Extreme Fear",
            25..=44 => "Fear",
            45..=55 => "Neutral",
            56..=74 => "Greed",
            _ => "Extreme Greed",
        };
        Ok(FearGreedSnapshot {
            value: self.fear_greed,
            classification: classification.to_string(),
            updated_ms: now_ms(),
        })
    }

    async fn dex_liquidity(&self, asset: &Asset) -> Result<DexLiquidity, CmcError> {
        let s = seed(&asset.symbol);
        let liq = d(2_000_000 + s * 3000, 0);
        Ok(DexLiquidity {
            symbol: asset.symbol.clone(),
            total_liquidity_usd: liq,
            top_pair_liquidity_usd: liq / Decimal::from(2),
            pair_count: 3 + (s % 5) as u32,
        })
    }

    async fn token_security(&self, asset: &Asset) -> Result<TokenSecurity, CmcError> {
        let s = seed(&asset.symbol);
        // Inject a flag occasionally so the security check has something to do.
        let flags = if s % 97 == 0 {
            vec!["low_holder_count".to_string()]
        } else {
            vec![]
        };
        Ok(TokenSecurity {
            symbol: asset.symbol.clone(),
            flags,
            safety_score: 70 + (s % 30) as u32,
        })
    }

    async fn trending(&self) -> Result<Vec<TrendingToken>, CmcError> {
        Ok(vec![
            TrendingToken {
                cmc_id: 7186,
                symbol: "CAKE".into(),
                rank: 1,
            },
            TrendingToken {
                cmc_id: 825,
                symbol: "USDT".into(),
                rank: 2,
            },
        ])
    }

    async fn global_market(&self) -> Result<GlobalMarket, CmcError> {
        Ok(GlobalMarket {
            total_market_cap_usd: d(2_400_000_000_000, 0),
            btc_dominance_pct: d(5230, 2),
            updated_ms: now_ms(),
        })
    }
}
