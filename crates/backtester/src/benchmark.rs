//! Buy-and-hold benchmark for backtests.
//!
//! The benchmark allocates the starting capital equally across the eligible
//! non-stable assets at the first step and simply holds. Comparing the
//! strategy's return against this isolates the strategy's contribution from the
//! market's drift.

use common::Decimal;
use std::collections::HashMap;

/// Default single-asset benchmark symbol (used by simple comparisons).
pub const BENCHMARK_SYMBOL: &str = "WBNB";

/// Tracks an equal-weight buy-and-hold basket established at the first step.
#[derive(Debug, Default)]
pub struct BuyAndHold {
    /// symbol -> quantity held
    quantities: HashMap<String, Decimal>,
    established: bool,
}

impl BuyAndHold {
    pub fn new() -> Self {
        BuyAndHold::default()
    }

    /// Establish the basket once: split `capital_usd` equally across the given
    /// non-stable symbols at their current prices. No-op after the first call.
    pub fn establish(
        &mut self,
        capital_usd: Decimal,
        prices: &HashMap<String, Decimal>,
        symbols: &[String],
    ) {
        if self.established || symbols.is_empty() {
            return;
        }
        let per_asset = capital_usd / Decimal::from(symbols.len() as i64);
        for symbol in symbols {
            if let Some(price) = prices.get(symbol) {
                if *price > Decimal::ZERO {
                    self.quantities.insert(symbol.clone(), per_asset / *price);
                }
            }
        }
        self.established = true;
    }

    /// Mark-to-market value of the basket at the given prices.
    pub fn value(&self, prices: &HashMap<String, Decimal>) -> Decimal {
        self.quantities
            .iter()
            .map(|(symbol, qty)| *qty * prices.get(symbol).copied().unwrap_or(Decimal::ZERO))
            .sum()
    }
}

/// Percent return from `start` to `end`.
pub fn return_pct(start: Decimal, end: Decimal) -> Decimal {
    if start <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    ((end - start) / start * Decimal::from(100)).round_dp(3)
}
