//! Deterministic synthetic market generator for backtesting.
//!
//! Produces an evolving, multi-phase price path (advances and pullbacks) so the
//! backtest exercises real drawdown, rebalancing, and risk behaviour rather
//! than a monotone line. The same generator seeds the CLI `score` command.

use common::time::now_ms;
use common::{Asset, Decimal};
use market_data::{AssetMarketState, FearGreedSnapshot, GlobalMarketState, MarketSnapshot};
use std::collections::HashMap;

/// Cheap deterministic hash of a symbol into a 0..1000 bucket.
fn seed(symbol: &str) -> i64 {
    symbol
        .bytes()
        .fold(0i64, |acc, b| (acc * 31 + b as i64) % 1000)
}

/// Strength tier (1..5) derived from the symbol — separates leaders from laggards.
fn lead(symbol: &str) -> i64 {
    seed(symbol) % 5 + 1
}

/// Starting price for a non-stable asset, in USD.
pub fn initial_price(symbol: &str) -> Decimal {
    Decimal::new(100 + seed(symbol), 2) // $1.00 .. $11.00
}

/// The 24h return (percent) for a symbol at a given step.
///
/// Combines two parts:
/// - an oscillation (near-zero-mean 8-phase wave) for volatility, scaled by the
///   symbol's strength tier and offset per symbol so the book is decorrelated;
/// - a sentiment drift derived from `fear_greed`: fearful markets trend down,
///   greedy markets trend up. This lets the regime-routed strategy demonstrate
///   capital preservation (positive excess vs buy-and-hold) in down markets.
pub fn step_return_24h_pct(symbol: &str, step: u32, fear_greed: u32) -> Decimal {
    // Near-zero-mean oscillation (basis points) — pure volatility.
    const OSC_BP: [i64; 8] = [300, -200, 400, -300, 200, -400, 300, -150];
    let l = lead(symbol);
    let idx = ((step as i64 + l) % 8) as usize;
    let osc = OSC_BP[idx] * (8 + l) / 10;
    // Sentiment drift: ~ -2.5%/step at F&G 0, flat at 50, +2.5%/step at 100.
    let drift = (fear_greed as i64 - 50) * 5;
    Decimal::new(osc + drift, 2)
}

/// Build a full market snapshot for `step` given the currently-evolved prices.
pub fn build_snapshot(
    universe: &[Asset],
    prices: &HashMap<String, Decimal>,
    step: u32,
    fear_greed: u32,
) -> MarketSnapshot {
    let mut assets = Vec::with_capacity(universe.len());
    for asset in universe {
        let is_stable = asset.category.is_stable();
        let s = seed(&asset.symbol);
        let price = prices.get(&asset.symbol).copied().unwrap_or(Decimal::ONE);
        let r24h = if is_stable {
            Decimal::ZERO
        } else {
            step_return_24h_pct(&asset.symbol, step, fear_greed)
        };
        let r1h = r24h / Decimal::from(4);

        assets.push(AssetMarketState {
            asset: asset.clone(),
            price_usd: price,
            volume_24h_usd: Decimal::new(8_000_000 + s * 20_000, 0),
            market_cap_usd: Some(Decimal::new(50_000_000 + s * 100_000, 0)),
            liquidity_usd: Some(Decimal::new(2_000_000 + s * 3000, 0)),
            ret_1h: Some(r1h),
            ret_24h: Some(r24h),
            // ~3% hourly range — the strategy's volatility sweet spot.
            volatility_1h: Some(Decimal::new(3, 0)),
            safety_score: 80,
            security_flags: vec![],
        });
    }

    MarketSnapshot {
        timestamp_ms: now_ms(),
        assets,
        fear_greed: Some(FearGreedSnapshot {
            value: fear_greed,
            classification: "Greed".to_string(),
            updated_ms: now_ms(),
        }),
        global_market: Some(GlobalMarketState {
            total_market_cap_usd: Decimal::new(2_400_000_000_000, 0),
            btc_dominance_pct: Decimal::new(5230, 2),
        }),
    }
}
