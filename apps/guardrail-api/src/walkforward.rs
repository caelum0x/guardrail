//! On-demand walk-forward analysis endpoint.
//!
//! Runs the real strategy + risk + portfolio pipeline across a sequence of
//! windows, each driven by its own fear/greed reading, and returns per-window
//! metrics plus an aggregate summary. Read-only and side-effect free — it never
//! touches the live book or the event log.

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

const UNIVERSE: &str = "configs/eligible_assets.bsc.json";
const POLICY: &str = "configs/risk_policy.paper.json";

/// Sentiment path cycled across windows.
const FEAR_GREED_PATH: [u32; 6] = [25, 40, 55, 70, 85, 60];

#[derive(Debug, Deserialize)]
pub struct WalkForwardParams {
    /// Number of windows to evaluate (default 6, clamped to 1..24).
    pub windows: Option<u32>,
    /// Steps per window (default 30, clamped to 1..500).
    pub steps: Option<u32>,
    /// Strategy preset name (default "balanced").
    pub preset: Option<String>,
}

pub async fn walkforward(Query(params): Query<WalkForwardParams>) -> Json<Value> {
    match run(&params) {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn run(params: &WalkForwardParams) -> anyhow::Result<Value> {
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
    let cfg = backtester::WalkForwardConfig {
        windows: params.windows.unwrap_or(6).clamp(1, 24),
        steps_per_window: params.steps.unwrap_or(30).clamp(1, 500),
        fear_greed_path: FEAR_GREED_PATH.to_vec(),
    };

    let report = backtester::walk_forward(&universe, policy, strat_cfg, cfg);

    let windows: Vec<Value> = report
        .windows
        .iter()
        .map(|w| {
            json!({
                "window": w.window,
                "fear_greed": w.fear_greed,
                "total_return_pct": w.total_return_pct.to_string(),
                "max_drawdown_pct": w.max_drawdown_pct.to_string(),
                "benchmark_return_pct": w.benchmark_return_pct.to_string(),
                "excess_return_pct": w.excess_return_pct.to_string(),
                "trades": w.trades,
            })
        })
        .collect();

    Ok(json!({
        "preset": crate::presets::requested_name(preset),
        "windows": windows,
        "aggregate": {
            "mean_excess_pct": report.mean_excess_pct.to_string(),
            "worst_drawdown_pct": report.worst_drawdown_pct.to_string(),
            "positive_windows": report.positive_windows,
            "total": report.windows.len(),
        },
    }))
}
