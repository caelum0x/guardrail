//! Per-skill detail + on-demand backtest: `GET /skills/{id}` and
//! `GET /skills/{id}/backtest?preset=`.
//!
//! Uses the `skill-loader` crate to resolve a skill from `skills/INDEX.json`,
//! returns its catalog entry + spec summary, and (for the backtest route) runs
//! the real strategy + risk + portfolio pipeline over the eligible universe with
//! the requested preset. Read-only and panic-free; an unknown id returns a
//! 404-style JSON body.

use axum::extract::{Path, Query};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path as FsPath;

const UNIVERSE: &str = "configs/eligible_assets.bsc.json";
const POLICY: &str = "configs/risk_policy.paper.json";
const REPO_ROOT: &str = ".";

#[derive(Debug, Deserialize)]
pub struct BacktestParams {
    pub steps: Option<u32>,
    pub fear_greed: Option<u32>,
    pub preset: Option<String>,
}

/// GET /skills/{id} — catalog entry + spec section summary.
pub async fn skill_detail(Path(id): Path<String>) -> Json<Value> {
    let catalog = match skill_loader::SkillCatalog::load(FsPath::new(REPO_ROOT)) {
        Ok(c) => c,
        Err(e) => return Json(json!({ "error": format!("catalog load failed: {e}") })),
    };
    let Some(entry) = catalog.get(&id) else {
        return Json(json!({ "error": "skill not found", "id": id }));
    };

    let examples_on_disk = entry.count_examples_on_disk(catalog.root());
    let spec = entry.load_spec(catalog.root()).ok();
    let spec_sections: Vec<String> = spec
        .as_ref()
        .and_then(|s| s.body.as_mapping())
        .map(|m| {
            m.keys()
                .filter_map(|k| k.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let description = spec.as_ref().and_then(|s| s.header.description.clone());

    Json(json!({
        "id": entry.id,
        "name": entry.name,
        "summary": entry.summary,
        "description": description,
        "regimes": entry.regimes,
        "inputs": entry.inputs,
        "eligible_universe_size": entry.eligible_universe_size,
        "examples_count": entry.examples_count,
        "examples_on_disk": examples_on_disk,
        "spec_file": entry.spec_file,
        "spec_sections": spec_sections,
    }))
}

/// GET /skills/{id}/backtest?preset= — run a backtest contextualized by the skill.
pub async fn skill_backtest(Path(id): Path<String>, Query(params): Query<BacktestParams>) -> Json<Value> {
    let catalog = match skill_loader::SkillCatalog::load(FsPath::new(REPO_ROOT)) {
        Ok(c) => c,
        Err(e) => return Json(json!({ "error": format!("catalog load failed: {e}") })),
    };
    let Some(entry) = catalog.get(&id) else {
        return Json(json!({ "error": "skill not found", "id": id }));
    };
    let regimes = entry.regimes.clone();

    match run_backtest(&params) {
        Ok(mut result) => {
            if let Value::Object(ref mut map) = result {
                map.insert("skill_id".into(), json!(id));
                map.insert("skill_regimes".into(), json!(regimes));
                map.insert(
                    "note".into(),
                    json!("backtest over the shared eligible universe with the selected preset; \
                           the skill's regime routing is documented in its strategy_spec.yaml"),
                );
            }
            Json(result)
        }
        Err(e) => Json(json!({ "error": e.to_string(), "skill_id": id })),
    }
}

fn run_backtest(params: &BacktestParams) -> anyhow::Result<Value> {
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
    Ok(json!({
        "preset": crate::presets::requested_name(preset),
        "steps": result.steps,
        "final_nav_usd": result.final_nav_usd.round_dp(2).to_string(),
        "benchmark_return_pct": result.benchmark_return_pct.to_string(),
        "excess_return_pct": result.excess_return_pct.to_string(),
        "metrics": {
            "total_return_pct": result.metrics.total_return_pct.to_string(),
            "max_drawdown_pct": result.metrics.max_drawdown_pct.to_string(),
            "trade_count": result.metrics.trade_count,
            "calmar_ratio": result.metrics.calmar_ratio.to_string(),
        },
    }))
}
