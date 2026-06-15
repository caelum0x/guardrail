use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Candle interval requested from the OHLCV endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Interval {
    M15,
    H1,
    H4,
    D1,
}

impl Interval {
    pub fn as_str(&self) -> &'static str {
        match self {
            Interval::M15 => "15m",
            Interval::H1 => "1h",
            Interval::H4 => "4h",
            Interval::D1 => "1d",
        }
    }
}

/// A normalized latest-quote row for one asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmcQuote {
    pub cmc_id: u64,
    pub symbol: String,
    pub price_usd: Decimal,
    pub volume_24h_usd: Decimal,
    pub market_cap_usd: Option<Decimal>,
    pub percent_change_1h: Option<Decimal>,
    pub percent_change_24h: Option<Decimal>,
    pub last_updated_ms: i64,
}

/// One OHLCV candle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub open_time_ms: i64,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

/// CMC Fear & Greed index snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearGreedSnapshot {
    /// 0..=100
    pub value: u32,
    /// e.g. "Fear", "Greed", "Neutral"
    pub classification: String,
    pub updated_ms: i64,
}

/// Aggregated DEX liquidity for an asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexLiquidity {
    pub symbol: String,
    pub total_liquidity_usd: Decimal,
    pub top_pair_liquidity_usd: Decimal,
    pub pair_count: u32,
}

/// Token security / risk signals (honeypot, mintable, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSecurity {
    pub symbol: String,
    pub flags: Vec<String>,
    /// 0..=100, higher is safer.
    pub safety_score: u32,
}

/// A trending token surfaced by CMC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingToken {
    pub cmc_id: u64,
    pub symbol: String,
    pub rank: u32,
}

/// Global market context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMarket {
    pub total_market_cap_usd: Decimal,
    pub btc_dominance_pct: Decimal,
    pub updated_ms: i64,
}
