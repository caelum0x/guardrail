use crate::holding::Holding;
use common::constants::RESERVE_SYMBOL;
use common::{Asset, AssetCategory, Decimal};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The current set of holdings plus realized PnL to date.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PortfolioState {
    pub holdings: Vec<Holding>,
    pub realized_pnl_usd: Decimal,
}

impl PortfolioState {
    pub fn new() -> Self {
        PortfolioState::default()
    }

    /// Seed a paper portfolio with an all-stable starting balance.
    pub fn seed_stable(amount_usd: Decimal) -> Self {
        PortfolioState {
            holdings: vec![Holding::new(RESERVE_SYMBOL, amount_usd, Decimal::ONE)],
            realized_pnl_usd: Decimal::ZERO,
        }
    }

    pub fn get(&self, symbol: &str) -> Option<&Holding> {
        self.holdings.iter().find(|h| h.symbol == symbol)
    }

    pub fn get_mut(&mut self, symbol: &str) -> Option<&mut Holding> {
        self.holdings.iter_mut().find(|h| h.symbol == symbol)
    }

    /// Total net asset value across all holdings, in USD.
    pub fn nav_usd(&self) -> Decimal {
        self.holdings.iter().map(|h| h.market_value_usd()).sum()
    }

    /// Value held in the stable reserve, in USD.
    pub fn stable_value_usd(&self) -> Decimal {
        self.get(RESERVE_SYMBOL)
            .map(|h| h.market_value_usd())
            .unwrap_or(Decimal::ZERO)
    }

    /// Stable reserve as a percent of NAV.
    pub fn stable_reserve_pct(&self) -> Decimal {
        let nav = self.nav_usd();
        if nav.is_zero() {
            return Decimal::ZERO;
        }
        self.stable_value_usd() / nav * Decimal::from(100)
    }

    /// Current weight of a symbol as a percent of NAV.
    pub fn weight_pct(&self, symbol: &str) -> Decimal {
        let nav = self.nav_usd();
        if nav.is_zero() {
            return Decimal::ZERO;
        }
        self.get(symbol)
            .map(|h| h.market_value_usd() / nav * Decimal::from(100))
            .unwrap_or(Decimal::ZERO)
    }

    /// All non-reserve weights, keyed by symbol.
    pub fn risk_weights_pct(&self) -> HashMap<String, Decimal> {
        let nav = self.nav_usd();
        let mut map = HashMap::new();
        if nav.is_zero() {
            return map;
        }
        for h in &self.holdings {
            if h.symbol == RESERVE_SYMBOL {
                continue;
            }
            map.insert(
                h.symbol.clone(),
                h.market_value_usd() / nav * Decimal::from(100),
            );
        }
        map
    }

    /// Update the mark price for a symbol.
    pub fn mark(&mut self, symbol: &str, price_usd: Decimal) {
        if let Some(h) = self.get_mut(symbol) {
            h.price_usd = price_usd;
        }
    }

    /// Mark every holding from an asset->price map.
    pub fn mark_all(&mut self, prices: &HashMap<String, Decimal>) {
        for h in self.holdings.iter_mut() {
            if let Some(p) = prices.get(&h.symbol) {
                h.price_usd = *p;
            }
        }
    }

    /// Number of distinct non-reserve positions held.
    pub fn position_count(&self) -> usize {
        self.holdings
            .iter()
            .filter(|h| h.symbol != RESERVE_SYMBOL && h.quantity > Decimal::ZERO)
            .count()
    }

    /// Convenience to register an asset's category presence (no-op hook for
    /// future per-category exposure caps).
    pub fn is_stable_symbol(asset: &Asset) -> bool {
        asset.category == AssetCategory::Stable
    }
}
