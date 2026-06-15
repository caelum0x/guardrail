//! Sentiment sweep and cross-preset comparison over the real backtest engine.

use backtester::{run_backtest, BacktestConfig};
use market_data::Universe;
use risk_engine::RiskPolicy;
use rust_decimal::Decimal;
use serde::Serialize;
use strategy_engine::StrategyConfig;

use crate::preset;

/// One row of a sentiment sweep: the outcome at a single Fear & Greed reading.
#[derive(Debug, Clone, Serialize)]
pub struct SweepRow {
    pub fear_greed: u32,
    pub return_pct: Decimal,
    pub benchmark_pct: Decimal,
    pub excess_pct: Decimal,
    pub max_drawdown_pct: Decimal,
    pub trades: u64,
}

/// Run the backtest once per Fear & Greed reading, holding strategy/policy fixed.
pub fn run_sweep(
    universe: &Universe,
    policy: &RiskPolicy,
    cfg: &StrategyConfig,
    steps: u32,
    fg_values: &[u32],
    starting_usd: u64,
) -> Vec<SweepRow> {
    fg_values
        .iter()
        .map(|&fg| {
            let run = run_backtest(
                universe,
                policy.clone(),
                cfg.clone(),
                BacktestConfig {
                    steps,
                    starting_usd: Decimal::from(starting_usd),
                    fear_greed: fg,
                },
            );
            SweepRow {
                fear_greed: fg,
                return_pct: run.metrics.total_return_pct,
                benchmark_pct: run.benchmark_return_pct,
                excess_pct: run.excess_return_pct,
                max_drawdown_pct: run.metrics.max_drawdown_pct,
                trades: run.metrics.trade_count,
            }
        })
        .collect()
}

/// Aggregate outcome of one preset across the whole sentiment sweep.
#[derive(Debug, Clone, Serialize)]
pub struct PresetSummary {
    pub preset: String,
    pub mean_excess_pct: Decimal,
    pub worst_drawdown_pct: Decimal,
    pub total_trades: u64,
}

/// Run the sweep for every preset in the presets file and rank them by mean
/// excess return (best first). Returns an empty vec when no presets load.
pub fn compare_presets(
    universe: &Universe,
    policy: &RiskPolicy,
    cap: f64,
    steps: u32,
    fg_values: &[u32],
    starting_usd: u64,
) -> Vec<PresetSummary> {
    let presets = match preset::load_presets() {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let mut names: Vec<&String> = presets.keys().collect();
    names.sort();

    let mut summaries: Vec<PresetSummary> = names
        .into_iter()
        .map(|name| {
            let cfg = presets[name].apply(preset::base_config(cap));
            let rows = run_sweep(universe, policy, &cfg, steps, fg_values, starting_usd);
            summarize(name, &rows)
        })
        .collect();

    // Best mean excess first.
    summaries.sort_by_key(|s| std::cmp::Reverse(s.mean_excess_pct));
    summaries
}

/// Reduce a set of sweep rows into a single preset summary.
fn summarize(preset: &str, rows: &[SweepRow]) -> PresetSummary {
    let n = rows.len().max(1);
    let sum_excess: Decimal = rows.iter().map(|r| r.excess_pct).sum();
    let worst_drawdown = rows
        .iter()
        .map(|r| r.max_drawdown_pct)
        .max()
        .unwrap_or(Decimal::ZERO);
    PresetSummary {
        preset: preset.to_string(),
        mean_excess_pct: sum_excess / Decimal::from(n as u64),
        worst_drawdown_pct: worst_drawdown,
        total_trades: rows.iter().map(|r| r.trades).sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(fg: u32, excess: i64, dd: i64, trades: u64) -> SweepRow {
        SweepRow {
            fear_greed: fg,
            return_pct: Decimal::ZERO,
            benchmark_pct: Decimal::ZERO,
            excess_pct: Decimal::from(excess),
            max_drawdown_pct: Decimal::from(dd),
            trades,
        }
    }

    #[test]
    fn summarize_averages_excess_and_takes_worst_drawdown() {
        let rows = vec![row(20, 2, 5, 3), row(50, 4, 9, 4)];
        let s = summarize("balanced", &rows);
        assert_eq!(s.mean_excess_pct, Decimal::from(3));
        assert_eq!(s.worst_drawdown_pct, Decimal::from(9));
        assert_eq!(s.total_trades, 7);
    }
}
