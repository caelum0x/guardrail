//! Rebalance preview endpoint.
//!
//! Builds a current strategy decision against the paper configuration and the
//! latest run report positions. This is read-only: proposed orders remain
//! intents and still require the normal risk, quote, and TWAK execution path.

use axum::extract::Query;
use axum::Json;
use common::decimal::to_f64;
use common::{Decimal, Settings};
use serde::Deserialize;
use serde_json::{json, Value};
use std::str::FromStr;
use strategy_engine::{CurrentAllocation, StrategyEngine};

const UNIVERSE: &str = "configs/eligible_assets.bsc.json";
const PAPER_CONFIG: &str = "configs/paper.toml";
const DEFAULT_REPORT: &str = "data/run_report.json";
const DEFAULT_NAV_USD: i64 = 10_000;

#[derive(Debug, Deserialize)]
pub struct RebalanceParams {
    /// Strategy preset name (default "balanced").
    pub preset: Option<String>,
    /// Override NAV for what-if sizing.
    pub nav_usd: Option<String>,
}

pub async fn rebalance(Query(params): Query<RebalanceParams>) -> Json<Value> {
    match build(&params).await {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

async fn build(params: &RebalanceParams) -> anyhow::Result<Value> {
    let settings = Settings::load(PAPER_CONFIG)?;
    let universe = market_data::Universe::load(UNIVERSE)?;
    let policy_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let policy = risk_engine::RiskPolicy::from_json_str(&policy_raw)?;
    let position_cap_pct = (to_f64(policy.max_position_pct) - 1.0).max(1.0);

    let report_path = std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| DEFAULT_REPORT.into());
    let report = read_report(&report_path).unwrap_or_else(|| json!({}));
    let nav_usd = params
        .nav_usd
        .as_deref()
        .and_then(decimal_from_str)
        .or_else(|| {
            report
                .get("nav_usd")
                .and_then(Value::as_str)
                .and_then(decimal_from_str)
        })
        .unwrap_or_else(|| Decimal::from(DEFAULT_NAV_USD));
    let current = current_allocation_from_report(&report);

    let assets = universe.enabled_assets();
    let source = cmc_client::MockCmcClient::new();
    let snapshot = market_data::SnapshotBuilder::new(&source, &universe)
        .build()
        .await?;

    let overrides = crate::presets::resolve(params.preset.as_deref());
    let base_cfg = strategy_engine::StrategyConfig {
        max_positions: settings.strategy.max_positions,
        min_score_to_enter: settings.strategy.min_score_to_enter,
        min_score_to_hold: settings.strategy.min_score_to_hold,
        rebalance_threshold_pct: to_f64(settings.strategy.rebalance_threshold_pct),
        target_stable_reserve_pct: settings.strategy.target_stable_reserve_pct,
        max_position_weight_pct: position_cap_pct,
        ..Default::default()
    };
    let cfg = crate::presets::apply(base_cfg, &overrides);
    let strategy = StrategyEngine::new(cfg.clone());
    let decision = strategy.decide(&snapshot, &current, nav_usd);

    let current_weights: Vec<Value> = current
        .weights_pct
        .iter()
        .map(|(symbol, weight)| {
            json!({
                "symbol": symbol,
                "weight_pct": weight.round_dp(2).to_string()
            })
        })
        .collect();
    let deltas: Vec<Value> = decision
        .target_positions
        .iter()
        .map(|target| {
            let current_weight = current.weight(&target.symbol);
            json!({
                "symbol": target.symbol,
                "current_weight_pct": current_weight.round_dp(2).to_string(),
                "target_weight_pct": target.weight_pct.round_dp(2).to_string(),
                "delta_pct": (target.weight_pct - current_weight).round_dp(2).to_string()
            })
        })
        .collect();
    let largest_order_usd = decision
        .proposed_orders
        .iter()
        .map(|order| order.amount_usd)
        .max()
        .unwrap_or(Decimal::ZERO);

    Ok(json!({
        "preview_only": true,
        "preset": crate::presets::requested_name(params.preset.as_deref()),
        "report_path": report_path,
        "eligible_assets": assets.len(),
        "nav_usd": nav_usd.round_dp(2).to_string(),
        "regime": decision.regime.as_str(),
        "exposure_multiplier": decision.regime.exposure_multiplier().to_string(),
        "thresholds": {
            "rebalance_threshold_pct": cfg.rebalance_threshold_pct,
            "max_positions": cfg.max_positions,
            "max_position_weight_pct": cfg.max_position_weight_pct,
            "target_stable_reserve_pct": cfg.target_stable_reserve_pct
        },
        "summary": {
            "target_count": decision.target_positions.len(),
            "proposed_orders": decision.proposed_orders.len(),
            "largest_order_usd": largest_order_usd.round_dp(2).to_string(),
            "requires_risk_gate": !decision.proposed_orders.is_empty()
        },
        "explanation": {
            "headline": decision.explanation.headline,
            "top_scores": decision.explanation.top_scores,
            "target_summary": decision.explanation.target_summary,
            "order_count": decision.explanation.order_count,
            "fear_greed": decision.explanation.fear_greed
        },
        "current_weights": current_weights,
        "deltas": deltas,
        "targets": decision.target_positions,
        "orders": decision.proposed_orders
    }))
}

fn read_report(path: &str) -> Option<Value> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn current_allocation_from_report(report: &Value) -> CurrentAllocation {
    let mut current = CurrentAllocation::new();
    if let Some(positions) = report.get("positions").and_then(Value::as_array) {
        for position in positions {
            let Some(symbol) = position.get("symbol").and_then(Value::as_str) else {
                continue;
            };
            let Some(weight) = position
                .get("weight_pct")
                .and_then(Value::as_str)
                .and_then(decimal_from_str)
            else {
                continue;
            };
            current = current.with_weight(symbol, weight);
        }
    }
    current
}

fn decimal_from_str(value: &str) -> Option<Decimal> {
    Decimal::from_str(value).ok()
}
