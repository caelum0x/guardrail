//! Walk-forward analysis: run the real backtest engine across a sequence of
//! windows, each driven by its own sentiment (fear/greed) reading, then
//! aggregate per-window performance into a single report.
//!
//! Each window is an independent backtest over `steps_per_window` steps with a
//! fresh $10k starting NAV. The `fear_greed_path` supplies the sentiment for
//! each window and is cycled when shorter than the number of windows.

use crate::engine::{run_backtest, BacktestConfig};
use common::Decimal;
use market_data::Universe;
use risk_engine::RiskPolicy;
use serde::{Deserialize, Serialize};
use strategy_engine::StrategyConfig;

/// Default fear/greed reading used when the configured path is empty.
const DEFAULT_FEAR_GREED: u32 = 50;

/// Starting NAV for each individual walk-forward window.
fn window_starting_usd() -> Decimal {
    Decimal::from(10_000)
}

/// Configuration for a walk-forward run.
#[derive(Debug, Clone)]
pub struct WalkForwardConfig {
    /// Number of sequential windows to evaluate.
    pub windows: u32,
    /// Steps (24h periods) per window passed to the backtest engine.
    pub steps_per_window: u32,
    /// Sentiment reading for each window; cycled when shorter than `windows`.
    pub fear_greed_path: Vec<u32>,
}

/// Performance of a single walk-forward window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowResult {
    pub window: u32,
    pub fear_greed: u32,
    pub total_return_pct: Decimal,
    pub max_drawdown_pct: Decimal,
    pub benchmark_return_pct: Decimal,
    pub excess_return_pct: Decimal,
    pub trades: u64,
}

/// Aggregated report across all walk-forward windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardReport {
    pub windows: Vec<WindowResult>,
    pub mean_excess_pct: Decimal,
    pub worst_drawdown_pct: Decimal,
    pub positive_windows: u32,
}

/// Pick the fear/greed reading for a window, cycling the path when needed.
fn fear_greed_for(path: &[u32], window: u32) -> u32 {
    if path.is_empty() {
        return DEFAULT_FEAR_GREED;
    }
    path[(window as usize) % path.len()]
}

/// Run the backtest engine once per window and aggregate the results.
pub fn walk_forward(
    universe: &Universe,
    policy: RiskPolicy,
    strat_cfg: StrategyConfig,
    cfg: WalkForwardConfig,
) -> WalkForwardReport {
    let windows: Vec<WindowResult> = (0..cfg.windows)
        .map(|window| {
            let fear_greed = fear_greed_for(&cfg.fear_greed_path, window);
            let run = run_backtest(
                universe,
                policy.clone(),
                strat_cfg.clone(),
                BacktestConfig {
                    steps: cfg.steps_per_window,
                    starting_usd: window_starting_usd(),
                    fear_greed,
                },
            );
            WindowResult {
                window,
                fear_greed,
                total_return_pct: run.metrics.total_return_pct,
                max_drawdown_pct: run.metrics.max_drawdown_pct,
                benchmark_return_pct: run.benchmark_return_pct,
                excess_return_pct: run.excess_return_pct,
                trades: run.metrics.trade_count,
            }
        })
        .collect();

    let positive_windows = windows
        .iter()
        .filter(|w| w.excess_return_pct > Decimal::ZERO)
        .count() as u32;

    let worst_drawdown_pct = windows
        .iter()
        .map(|w| w.max_drawdown_pct)
        .max()
        .unwrap_or(Decimal::ZERO);

    let mean_excess_pct = if windows.is_empty() {
        Decimal::ZERO
    } else {
        let total: Decimal = windows.iter().map(|w| w.excess_return_pct).sum();
        (total / Decimal::from(windows.len() as u64)).round_dp(3)
    };

    WalkForwardReport {
        windows,
        mean_excess_pct,
        worst_drawdown_pct,
        positive_windows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{AssetCategory, EligibleAsset};

    fn asset(symbol: &str, category: AssetCategory) -> EligibleAsset {
        EligibleAsset {
            symbol: symbol.to_string(),
            cmc_id: 1,
            chain_id: 56,
            contract_address: format!("0x{symbol}"),
            decimals: 18,
            category,
            enabled: true,
            min_liquidity_usd: Decimal::ZERO,
            min_volume_24h_usd: Decimal::ZERO,
        }
    }

    fn test_universe() -> Universe {
        Universe::new(vec![
            asset("USDT", AssetCategory::Stable),
            asset("WBNB", AssetCategory::Core),
            asset("CAKE", AssetCategory::DeFi),
        ])
    }

    #[test]
    fn runs_one_window_per_config() {
        let universe = test_universe();
        let cfg = WalkForwardConfig {
            windows: 4,
            steps_per_window: 20,
            fear_greed_path: vec![25, 75],
        };

        let report = walk_forward(
            &universe,
            RiskPolicy::default(),
            StrategyConfig::default(),
            cfg,
        );

        assert_eq!(report.windows.len(), 4);
        assert!(report.positive_windows <= 4);

        for (i, w) in report.windows.iter().enumerate() {
            assert_eq!(w.window, i as u32);
        }
    }

    #[test]
    fn cycles_fear_greed_path() {
        let universe = test_universe();
        let cfg = WalkForwardConfig {
            windows: 5,
            steps_per_window: 10,
            fear_greed_path: vec![20, 80],
        };

        let report = walk_forward(
            &universe,
            RiskPolicy::default(),
            StrategyConfig::default(),
            cfg,
        );

        // Path [20, 80] cycled across 5 windows -> 20, 80, 20, 80, 20.
        let expected = [20, 80, 20, 80, 20];
        for (w, &fg) in report.windows.iter().zip(expected.iter()) {
            assert_eq!(w.fear_greed, fg);
        }
    }

    #[test]
    fn empty_path_uses_default_sentiment() {
        let universe = test_universe();
        let cfg = WalkForwardConfig {
            windows: 2,
            steps_per_window: 10,
            fear_greed_path: vec![],
        };

        let report = walk_forward(
            &universe,
            RiskPolicy::default(),
            StrategyConfig::default(),
            cfg,
        );

        assert_eq!(report.windows.len(), 2);
        for w in &report.windows {
            assert_eq!(w.fear_greed, DEFAULT_FEAR_GREED);
        }
        // worst_drawdown is non-negative by construction.
        assert!(report.worst_drawdown_pct >= Decimal::ZERO);
    }
}
