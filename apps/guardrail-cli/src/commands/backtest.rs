//! Backtest and market-regime commands: full backtests, preset comparison,
//! walk-forward analysis, the current alpha-score view, regime classification,
//! and a per-asset funding-rate proxy. These run the strategy + risk pipeline
//! over deterministic synthetic market paths.

use crate::{
    apply_preset, build_warmed_snapshot, strategy_config, DEFAULT_UNIVERSE, SNAPSHOT_FEAR_GREED,
    SNAPSHOT_WARMUP_STEPS,
};
use common::decimal::to_f64;
use common::Settings;
use rust_decimal::Decimal;
use strategy_engine::{CurrentAllocation, StrategyEngine};

pub fn run_backtest(config: &str, steps: u32, preset: &str) -> anyhow::Result<()> {
    let settings = Settings::load(config)?;
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let policy_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let policy = risk_engine::RiskPolicy::from_json_str(&policy_raw)?;
    let cap = (to_f64(policy.max_position_pct) - 1.0).max(1.0);
    let strat_cfg = apply_preset(strategy_config(&settings, cap), preset);

    let cfg = backtester::BacktestConfig {
        steps,
        ..Default::default()
    };
    let run = backtester::run_backtest(&universe, policy, strat_cfg, cfg);
    println!("{}", backtester::report::markdown(&run));
    Ok(())
}

/// Presets compared side by side, in increasing-risk order.
const COMPARE_PRESETS: [&str; 3] = ["conservative", "balanced", "aggressive"];

/// A single row of the preset comparison table.
struct CompareRow {
    preset: String,
    return_pct: f64,
    benchmark_pct: f64,
    excess_pct: f64,
    max_drawdown_pct: f64,
    calmar_ratio: f64,
    trade_count: u64,
}

pub fn run_compare(config: &str, steps: u32, fear_greed: u32) -> anyhow::Result<()> {
    let settings = Settings::load(config)?;
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let policy_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let policy = risk_engine::RiskPolicy::from_json_str(&policy_raw)?;
    let cap = (to_f64(policy.max_position_pct) - 1.0).max(1.0);

    let rows: Vec<CompareRow> = COMPARE_PRESETS
        .iter()
        .map(|preset| {
            let strat_cfg = apply_preset(strategy_config(&settings, cap), preset);
            let cfg = backtester::BacktestConfig {
                steps,
                starting_usd: Decimal::from(10_000),
                fear_greed,
            };
            let run = backtester::run_backtest(&universe, policy.clone(), strat_cfg, cfg);
            CompareRow {
                preset: (*preset).to_string(),
                return_pct: to_f64(run.metrics.total_return_pct),
                benchmark_pct: to_f64(run.benchmark_return_pct),
                excess_pct: to_f64(run.excess_return_pct),
                max_drawdown_pct: to_f64(run.metrics.max_drawdown_pct),
                calmar_ratio: to_f64(run.metrics.calmar_ratio),
                trade_count: run.metrics.trade_count,
            }
        })
        .collect();

    println!("# Strategy Preset Comparison");
    println!();
    println!("steps: {steps} · fear/greed: {fear_greed} · starting: $10,000");
    println!();
    println!("| Preset | Return % | Buy&Hold % | Excess % | Max DD % | Calmar | Trades |");
    println!("|:-------|---------:|-----------:|---------:|---------:|-------:|-------:|");
    for row in &rows {
        println!(
            "| {:<12} | {:>8.2} | {:>10.2} | {:>8.2} | {:>8.2} | {:>6.2} | {:>6} |",
            row.preset,
            row.return_pct,
            row.benchmark_pct,
            row.excess_pct,
            row.max_drawdown_pct,
            row.calmar_ratio,
            row.trade_count,
        );
    }
    Ok(())
}

pub fn run_walk_forward(
    config: &str,
    windows: u32,
    steps: u32,
    preset: &str,
) -> anyhow::Result<()> {
    let settings = Settings::load(config)?;
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let policy_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let policy = risk_engine::RiskPolicy::from_json_str(&policy_raw)?;
    let cap = (to_f64(policy.max_position_pct) - 1.0).max(1.0);
    let strat_cfg = apply_preset(strategy_config(&settings, cap), preset);

    let cfg = backtester::WalkForwardConfig {
        windows,
        steps_per_window: steps,
        fear_greed_path: vec![25, 40, 55, 70, 85, 60],
    };
    let report = backtester::walk_forward(&universe, policy, strat_cfg, cfg);

    println!("# Walk-Forward Analysis");
    println!();
    println!("windows: {windows} · steps/window: {steps}");
    println!();
    println!("| Window | Fear/Greed | Return % | Benchmark % | Excess % | Max DD % | Trades |");
    println!("|-------:|-----------:|---------:|------------:|---------:|---------:|-------:|");
    for w in &report.windows {
        println!(
            "| {:>6} | {:>10} | {:>8.2} | {:>11.2} | {:>8.2} | {:>8.2} | {:>6} |",
            w.window,
            w.fear_greed,
            to_f64(w.total_return_pct),
            to_f64(w.benchmark_return_pct),
            to_f64(w.excess_return_pct),
            to_f64(w.max_drawdown_pct),
            w.trades,
        );
    }
    println!();
    println!("## Aggregate");
    println!();
    println!("- mean excess: {:.2}%", to_f64(report.mean_excess_pct));
    println!(
        "- worst drawdown: {:.2}%",
        to_f64(report.worst_drawdown_pct)
    );
    println!(
        "- positive windows: {}/{}",
        report.positive_windows,
        report.windows.len()
    );
    Ok(())
}

pub fn run_score(config: &str) -> anyhow::Result<()> {
    let settings = Settings::load(config)?;
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let assets = universe.enabled_assets();

    // Warm up a short synthetic path so momentum is meaningful, then snapshot.
    use std::collections::HashMap;
    let mut prices: HashMap<String, Decimal> = HashMap::new();
    for a in &assets {
        if !a.category.is_stable() {
            prices.insert(
                a.symbol.clone(),
                backtester::synthetic::initial_price(&a.symbol),
            );
        } else {
            prices.insert(a.symbol.clone(), Decimal::ONE);
        }
    }
    let warmup = 5u32;
    for step in 0..warmup {
        for a in &assets {
            if a.category.is_stable() {
                continue;
            }
            let r = backtester::synthetic::step_return_24h_pct(&a.symbol, step, 60);
            if let Some(p) = prices.get_mut(&a.symbol) {
                *p *= Decimal::ONE + r / Decimal::from(100);
            }
        }
    }
    let snapshot = backtester::synthetic::build_snapshot(&assets, &prices, warmup, 60);

    let cap = (to_f64(risk_engine::RiskPolicy::default().max_position_pct) - 1.0).max(1.0);
    let strategy = StrategyEngine::new(strategy_config(&settings, cap));
    let decision = strategy.decide(&snapshot, &CurrentAllocation::new(), Decimal::from(10_000));

    println!("regime: {}", decision.regime.as_str());
    println!("{}", decision.explanation.headline);
    println!("\ntop alpha scores:");
    for (symbol, score) in &decision.explanation.top_scores {
        println!("  {symbol:<8} {score:.3}");
    }
    println!("\ntarget portfolio:");
    for t in &decision.target_positions {
        println!("  {:<8} {}%", t.symbol, t.weight_pct);
    }
    Ok(())
}

/// Classify the current synthetic market regime and print its inputs + exposure.
pub fn run_regime(config: &str) -> anyhow::Result<()> {
    let _settings = Settings::load(config)?;
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let assets = universe.enabled_assets();

    let snapshot = build_warmed_snapshot(&assets, SNAPSHOT_WARMUP_STEPS);
    let inputs = market_data::RegimeInputs::from_snapshot(&snapshot);
    let regime = strategy_engine::regime::classify(&inputs);

    println!("regime: {}", regime.as_str());
    println!("exposure multiplier: {:.2}", regime.exposure_multiplier());
    println!();
    println!("inputs:");
    println!("  fear_greed         : {}", inputs.fear_greed);
    println!("  breadth %          : {:.2}", to_f64(inputs.breadth_pct));
    println!(
        "  btc_dominance %    : {:.2}",
        to_f64(inputs.btc_dominance_pct)
    );
    println!(
        "  median 24h return %: {:.2}",
        to_f64(inputs.median_24h_return)
    );
    Ok(())
}

/// Funding-rate proxy bounds.
const FUNDING_PROXY_MIN: f64 = -1.0;
const FUNDING_PROXY_MAX: f64 = 1.0;

/// Print a per-asset funding-rate proxy table over a warmed synthetic snapshot.
///
/// The proxy is `ret_24h/24 + (volatility_1h - 3) * 0.01`, clamped to [-1, 1],
/// computed for each non-stable asset. It is a deterministic stand-in for a
/// perpetual funding rate while paper mode stays offline.
pub fn run_funding(steps: u32) -> anyhow::Result<()> {
    if steps == 0 {
        anyhow::bail!("steps must be greater than 0");
    }

    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let assets = universe.enabled_assets();
    let snapshot = build_warmed_snapshot(&assets, steps);

    println!("# Funding-Rate Proxy");
    println!();
    println!("steps: {steps} · fear/greed: {SNAPSHOT_FEAR_GREED}");
    println!();
    println!("{:<8} | {:>8} | {:>14}", "SYMBOL", "24H%", "FUNDING_PROXY");
    println!("{}", "-".repeat(36));

    for a in &snapshot.assets {
        if a.asset.category.is_stable() {
            continue;
        }
        let ret_24h = a.ret_24h.map(to_f64).unwrap_or(0.0);
        let vol_1h = a.volatility_1h.map(to_f64).unwrap_or(0.0);
        let proxy =
            (ret_24h / 24.0 + (vol_1h - 3.0) * 0.01).clamp(FUNDING_PROXY_MIN, FUNDING_PROXY_MAX);
        println!("{:<8} | {:>8.2} | {:>14.4}", a.asset.symbol, ret_24h, proxy);
    }
    Ok(())
}
