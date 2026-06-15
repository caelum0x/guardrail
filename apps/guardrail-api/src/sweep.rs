//! On-demand sentiment-sweep endpoint.
//!
//! Runs the real strategy + risk + portfolio pipeline once per fear/greed
//! reading and returns a comparison row per reading. This shows how the
//! strategy preserves capital in fear and lags buy-and-hold in euphoria.
//! Read-only and side-effect free — it never touches the live book or the
//! event log.

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

const UNIVERSE: &str = "configs/eligible_assets.bsc.json";
const POLICY: &str = "configs/risk_policy.paper.json";

/// Default comma-separated fear/greed readings to sweep.
const DEFAULT_FEAR_GREED: &str = "20,40,60,80";
/// Maximum number of readings to evaluate in a single sweep.
const MAX_READINGS: usize = 12;

#[derive(Debug, Deserialize)]
pub struct SweepParams {
    /// Steps to simulate per reading (default 40, clamped to 1..500).
    pub steps: Option<u32>,
    /// Comma-separated fear/greed readings (default "20,40,60,80").
    pub fear_greed: Option<String>,
    /// Strategy preset name (default "balanced").
    pub preset: Option<String>,
}

pub async fn sweep(Query(params): Query<SweepParams>) -> Json<Value> {
    match run(&params) {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

/// Parse the comma-separated `fear_greed` parameter into clamped readings.
fn parse_fear_greed(raw: &str) -> Vec<u32> {
    raw.split(',')
        .filter_map(|piece| piece.trim().parse::<u32>().ok())
        .map(|value| value.min(100))
        .take(MAX_READINGS)
        .collect()
}

fn run(params: &SweepParams) -> anyhow::Result<Value> {
    let universe = market_data::Universe::load(UNIVERSE)?;
    let policy_raw = std::fs::read_to_string(POLICY)?;
    let policy = risk_engine::RiskPolicy::from_json_str(&policy_raw)?;
    let cap = (common::decimal::to_f64(policy.max_position_pct) - 1.0).max(1.0);

    let preset = params.preset.as_deref();
    let overrides = crate::presets::resolve(preset);

    let steps = params.steps.unwrap_or(40).clamp(1, 500);
    let raw = params.fear_greed.as_deref().unwrap_or(DEFAULT_FEAR_GREED);
    let readings = parse_fear_greed(raw);

    let rows: Vec<Value> = readings
        .iter()
        .map(|&fear_greed| {
            let base_cfg = strategy_engine::StrategyConfig {
                max_position_weight_pct: cap,
                min_score_to_enter: 0.55,
                min_score_to_hold: 0.45,
                ..Default::default()
            };
            let strat_cfg = crate::presets::apply(base_cfg, &overrides);
            let cfg = backtester::BacktestConfig {
                steps,
                starting_usd: rust_decimal::Decimal::from(10_000),
                fear_greed,
            };
            let result = backtester::run_backtest(&universe, policy.clone(), strat_cfg, cfg);
            json!({
                "fear_greed": fear_greed,
                "total_return_pct": result.metrics.total_return_pct.to_string(),
                "benchmark_return_pct": result.benchmark_return_pct.to_string(),
                "excess_return_pct": result.excess_return_pct.to_string(),
                "max_drawdown_pct": result.metrics.max_drawdown_pct.to_string(),
                "trade_count": result.metrics.trade_count,
            })
        })
        .collect();

    Ok(json!({
        "steps": steps,
        "preset": crate::presets::requested_name(preset),
        "rows": rows,
    }))
}
