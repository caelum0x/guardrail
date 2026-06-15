//! On-demand backtest endpoint.
//!
//! Runs the real strategy + risk + portfolio pipeline over a synthetic price
//! path and returns metrics plus the equity curve. Read-only and side-effect
//! free — it never touches the live book or the event log.

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

const UNIVERSE: &str = "configs/eligible_assets.bsc.json";
const POLICY: &str = "configs/risk_policy.paper.json";

#[derive(Debug, Deserialize)]
pub struct BacktestParams {
    /// Number of steps to simulate (default 60, capped at 1000).
    pub steps: Option<u32>,
    /// Fear & Greed value 0..100 driving the regime (default 60).
    pub fear_greed: Option<u32>,
    /// Strategy preset name (default "balanced").
    pub preset: Option<String>,
}

pub async fn backtest(Query(params): Query<BacktestParams>) -> Json<Value> {
    match run(&params) {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn run(params: &BacktestParams) -> anyhow::Result<Value> {
    let universe = market_data::Universe::load(UNIVERSE)?;
    let policy_raw = std::fs::read_to_string(POLICY)?;
    let policy = risk_engine::RiskPolicy::from_json_str(&policy_raw)?;
    let cap = (common::decimal::to_f64(policy.max_position_pct) - 1.0).max(1.0);

    let preset = params.preset.as_deref();
    let overrides = crate::presets::resolve(preset);
    let base_cfg = strategy_engine::StrategyConfig {
        max_position_weight_pct: cap,
        min_score_to_enter: 0.55,
        min_score_to_hold: 0.45,
        ..Default::default()
    };
    let strat_cfg = crate::presets::apply(base_cfg, &overrides);
    let cfg = backtester::BacktestConfig {
        steps: params.steps.unwrap_or(60).clamp(1, 1000),
        starting_usd: rust_decimal::Decimal::from(10_000),
        fear_greed: params.fear_greed.unwrap_or(60).min(100),
    };

    let result = backtester::run_backtest(&universe, policy, strat_cfg, cfg);
    let curve: Vec<String> = result
        .equity_curve
        .iter()
        .map(|d| d.round_dp(2).to_string())
        .collect();

    Ok(json!({
        "steps": result.steps,
        "preset": crate::presets::requested_name(preset),
        "fear_greed": params.fear_greed.unwrap_or(60).min(100),
        "starting_nav_usd": result.starting_nav_usd.to_string(),
        "final_nav_usd": result.final_nav_usd.round_dp(2).to_string(),
        "benchmark_return_pct": result.benchmark_return_pct.to_string(),
        "excess_return_pct": result.excess_return_pct.to_string(),
        "metrics": {
            "total_return_pct": result.metrics.total_return_pct.to_string(),
            "max_drawdown_pct": result.metrics.max_drawdown_pct.to_string(),
            "trade_count": result.metrics.trade_count,
            "win_rate_pct": result.metrics.win_rate_pct.to_string(),
            "profit_factor": result.metrics.profit_factor.to_string(),
            "volatility_pct": result.metrics.volatility_pct.to_string(),
            "calmar_ratio": result.metrics.calmar_ratio.to_string(),
        },
        "equity_curve": curve,
    }))
}
