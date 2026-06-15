//! Portfolio drift endpoint.
//!
//! Compares the latest report positions against a fresh strategy target and
//! reports whether the book is within the configured drift policy. Read-only.

use axum::Json;
use common::decimal::to_f64;
use common::{Decimal, Settings};
use rust_decimal::prelude::{FromPrimitive, FromStr};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use strategy_engine::{CurrentAllocation, StrategyEngine};

const DRIFT_POLICY: &str = "configs/drift/drift_policy.json";
const UNIVERSE: &str = "configs/eligible_assets.bsc.json";

pub async fn drift() -> Json<Value> {
    match build().await {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

async fn build() -> anyhow::Result<Value> {
    let policy: Value = serde_json::from_str(&std::fs::read_to_string(DRIFT_POLICY)?)?;
    let report_path = std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| {
        policy
            .get("report_path")
            .and_then(Value::as_str)
            .unwrap_or("data/run_report.json")
            .to_string()
    });
    let config_path = policy
        .get("config_path")
        .and_then(Value::as_str)
        .unwrap_or("configs/paper.toml");
    let warning_delta = decimal_config(&policy, "warning_delta_pct", Decimal::from(3));
    let critical_delta = decimal_config(&policy, "critical_delta_pct", Decimal::from(8));
    let max_turnover = decimal_config(&policy, "max_turnover_pct", Decimal::from(35));
    let stable_symbol = policy
        .get("stable_symbol")
        .and_then(Value::as_str)
        .unwrap_or("USDT");

    let report: Value = serde_json::from_str(&std::fs::read_to_string(&report_path)?)?;
    let nav = report
        .get("nav_usd")
        .and_then(Value::as_str)
        .and_then(decimal_from_str)
        .unwrap_or_else(|| Decimal::from(10_000));
    let current = current_allocation_from_report(&report);

    let settings = Settings::load(config_path)?;
    let universe = market_data::Universe::load(UNIVERSE)?;
    let risk_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let risk_policy = risk_engine::RiskPolicy::from_json_str(&risk_raw)?;
    let cap = (to_f64(risk_policy.max_position_pct) - 1.0).max(1.0);
    let cfg = strategy_engine::StrategyConfig {
        max_positions: settings.strategy.max_positions,
        min_score_to_enter: settings.strategy.min_score_to_enter,
        min_score_to_hold: settings.strategy.min_score_to_hold,
        rebalance_threshold_pct: to_f64(settings.strategy.rebalance_threshold_pct),
        target_stable_reserve_pct: settings.strategy.target_stable_reserve_pct,
        max_position_weight_pct: cap,
        ..Default::default()
    };
    let source = cmc_client::MockCmcClient::new();
    let snapshot = market_data::SnapshotBuilder::new(&source, &universe)
        .build()
        .await?;
    let decision = StrategyEngine::new(cfg).decide(&snapshot, &current, nav);

    let mut symbols = BTreeMap::<String, (Decimal, Decimal)>::new();
    for (symbol, current_weight) in &current.weights_pct {
        symbols.insert(symbol.clone(), (*current_weight, Decimal::ZERO));
    }
    for target in &decision.target_positions {
        symbols
            .entry(target.symbol.clone())
            .and_modify(|entry| entry.1 = target.weight_pct)
            .or_insert((Decimal::ZERO, target.weight_pct));
    }

    let mut rows = Vec::new();
    let mut max_abs_delta = Decimal::ZERO;
    let mut turnover = Decimal::ZERO;
    for (symbol, (current_weight, target_weight)) in symbols {
        let delta = target_weight - current_weight;
        let abs_delta = delta.abs();
        if abs_delta > max_abs_delta {
            max_abs_delta = abs_delta;
        }
        if symbol != stable_symbol {
            turnover += abs_delta;
        }
        let status = if abs_delta >= critical_delta {
            "critical"
        } else if abs_delta >= warning_delta {
            "watch"
        } else {
            "normal"
        };
        rows.push(json!({
            "symbol": symbol,
            "status": status,
            "current_weight_pct": current_weight.round_dp(2).to_string(),
            "target_weight_pct": target_weight.round_dp(2).to_string(),
            "delta_pct": delta.round_dp(2).to_string(),
            "abs_delta_pct": abs_delta.round_dp(2).to_string()
        }));
    }
    rows.sort_by_key(|row| std::cmp::Reverse(decimal_field(row, "abs_delta_pct")));
    let status = if max_abs_delta >= critical_delta || turnover > max_turnover {
        "critical"
    } else if max_abs_delta >= warning_delta {
        "watch"
    } else {
        "aligned"
    };

    Ok(json!({
        "status": status,
        "policy_path": DRIFT_POLICY,
        "report_path": report_path,
        "config_path": config_path,
        "regime": decision.regime.as_str(),
        "nav_usd": nav.round_dp(2).to_string(),
        "thresholds": {
            "warning_delta_pct": warning_delta.to_string(),
            "critical_delta_pct": critical_delta.to_string(),
            "max_turnover_pct": max_turnover.to_string()
        },
        "summary": {
            "positions": rows.len(),
            "max_abs_delta_pct": max_abs_delta.round_dp(2).to_string(),
            "turnover_pct": turnover.round_dp(2).to_string(),
            "turnover_usd": (nav * turnover / Decimal::from(100)).round_dp(2).to_string()
        },
        "positions": rows
    }))
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

fn decimal_config(config: &Value, key: &str, default: Decimal) -> Decimal {
    config
        .get(key)
        .and_then(Value::as_f64)
        .and_then(Decimal::from_f64)
        .or_else(|| config.get(key).and_then(Value::as_i64).map(Decimal::from))
        .or_else(|| config.get(key).and_then(Value::as_u64).map(Decimal::from))
        .or_else(|| {
            config
                .get(key)
                .and_then(Value::as_str)
                .and_then(decimal_from_str)
        })
        .unwrap_or(default)
}

fn decimal_field(value: &Value, field: &str) -> Decimal {
    value
        .get(field)
        .and_then(Value::as_str)
        .and_then(decimal_from_str)
        .unwrap_or(Decimal::ZERO)
}

fn decimal_from_str(value: &str) -> Option<Decimal> {
    Decimal::from_str(value).ok()
}
