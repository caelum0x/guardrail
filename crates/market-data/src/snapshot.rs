//! The normalized market snapshot and its builder.

use crate::universe::Universe;
use cmc_client::{CmcDataSource, FearGreedSnapshot, Interval};
use common::time::now_ms;
use common::{Asset, Decimal};
use serde::{Deserialize, Serialize};

/// Global market context attached to a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMarketState {
    pub total_market_cap_usd: Decimal,
    pub btc_dominance_pct: Decimal,
}

/// Per-asset normalized state. The strategy and risk engines read only this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMarketState {
    pub asset: Asset,
    pub price_usd: Decimal,
    pub volume_24h_usd: Decimal,
    pub market_cap_usd: Option<Decimal>,
    pub liquidity_usd: Option<Decimal>,
    pub ret_1h: Option<Decimal>,
    pub ret_24h: Option<Decimal>,
    pub volatility_1h: Option<Decimal>,
    pub safety_score: u32,
    pub security_flags: Vec<String>,
}

/// A validated, point-in-time view of the eligible universe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSnapshot {
    pub timestamp_ms: i64,
    pub assets: Vec<AssetMarketState>,
    pub fear_greed: Option<FearGreedSnapshot>,
    pub global_market: Option<GlobalMarketState>,
}

impl MarketSnapshot {
    pub fn get(&self, symbol: &str) -> Option<&AssetMarketState> {
        self.assets.iter().find(|a| a.asset.symbol == symbol)
    }

    pub fn age_ms(&self) -> i64 {
        now_ms() - self.timestamp_ms
    }
}

/// Pulls from a [`CmcDataSource`] and assembles a [`MarketSnapshot`].
///
/// `S` is `?Sized` so callers can pass a trait object (`&dyn CmcDataSource`),
/// letting the runtime swap the live REST client for the mock by config.
pub struct SnapshotBuilder<'a, S: CmcDataSource + ?Sized> {
    source: &'a S,
    universe: &'a Universe,
}

impl<'a, S: CmcDataSource + ?Sized> SnapshotBuilder<'a, S> {
    pub fn new(source: &'a S, universe: &'a Universe) -> Self {
        SnapshotBuilder { source, universe }
    }

    /// Build a full snapshot for all enabled assets.
    pub async fn build(&self) -> anyhow::Result<MarketSnapshot> {
        let assets = self.universe.enabled_assets();
        let quotes = self.source.latest_quotes(&assets).await?;
        let fear_greed = self.source.fear_greed().await.ok();
        let global = self.source.global_market().await.ok();

        let mut states = Vec::with_capacity(assets.len());
        for asset in &assets {
            let Some(quote) = quotes.iter().find(|q| q.cmc_id == asset.cmc_id) else {
                continue;
            };
            let liquidity = self.source.dex_liquidity(asset).await.ok();
            let security = self.source.token_security(asset).await.ok();
            let volatility = self.estimate_volatility(asset).await;

            states.push(AssetMarketState {
                asset: asset.clone(),
                price_usd: quote.price_usd,
                volume_24h_usd: quote.volume_24h_usd,
                market_cap_usd: quote.market_cap_usd,
                liquidity_usd: liquidity.map(|l| l.total_liquidity_usd),
                ret_1h: quote.percent_change_1h,
                ret_24h: quote.percent_change_24h,
                volatility_1h: volatility,
                safety_score: security.as_ref().map(|s| s.safety_score).unwrap_or(50),
                security_flags: security.map(|s| s.flags).unwrap_or_default(),
            });
        }

        Ok(MarketSnapshot {
            timestamp_ms: now_ms(),
            assets: states,
            fear_greed,
            global_market: global.map(|g| GlobalMarketState {
                total_market_cap_usd: g.total_market_cap_usd,
                btc_dominance_pct: g.btc_dominance_pct,
            }),
        })
    }

    /// Rough realized volatility from recent hourly candles (high-low range).
    async fn estimate_volatility(&self, asset: &Asset) -> Option<Decimal> {
        let candles = self.source.ohlcv(asset, Interval::H1).await.ok()?;
        if candles.is_empty() {
            return None;
        }
        let mut total = Decimal::ZERO;
        for c in &candles {
            if c.close > Decimal::ZERO {
                total += (c.high - c.low) / c.close;
            }
        }
        Some(total / Decimal::from(candles.len() as i64) * Decimal::from(100))
    }
}
