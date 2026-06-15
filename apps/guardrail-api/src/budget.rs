//! Trading budget and execution runway endpoint.
//!
//! Combines the latest run report with product-owned execution budget policy
//! and BSC cost assumptions. Read-only.

use axum::Json;
use common::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde_json::{json, Value};

const BUDGET_CONFIG: &str = "configs/budgets/trading_budget.json";

pub async fn budget() -> Json<Value> {
    match build() {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build() -> anyhow::Result<Value> {
    let config: Value = serde_json::from_str(&std::fs::read_to_string(BUDGET_CONFIG)?)?;
    let cost_path = config
        .get("cost_config_path")
        .and_then(Value::as_str)
        .unwrap_or("configs/costs/bsc_execution_costs.json");
    let report_path = std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| {
        config
            .get("report_path")
            .and_then(Value::as_str)
            .unwrap_or("data/run_report.json")
            .to_string()
    });
    let costs: Value = serde_json::from_str(&std::fs::read_to_string(cost_path)?)?;
    let report: Value = serde_json::from_str(&std::fs::read_to_string(&report_path)?)?;

    let nav_usd = decimal_value(report.get("nav_usd")).unwrap_or(Decimal::ZERO);
    let daily_trade_target =
        decimal_value(config.get("daily_trade_target")).unwrap_or(Decimal::ONE);
    let planned_days =
        decimal_value(config.get("planned_competition_days")).unwrap_or(Decimal::from(7));
    let gas_float = decimal_value(config.get("operator_gas_float_usd")).unwrap_or(Decimal::ZERO);
    let max_daily_cost =
        decimal_value(config.get("max_daily_execution_cost_usd")).unwrap_or(Decimal::from(5));
    let max_cost_bps =
        decimal_value(config.get("max_cost_bps_per_trade")).unwrap_or(Decimal::from(25));
    let min_nav = decimal_value(config.get("min_nav_usd")).unwrap_or(Decimal::ZERO);

    let amount =
        decimal_value(costs.get("default_order_notional_usd")).unwrap_or(Decimal::from(1000));
    let native_price = decimal_value(costs.get("native_price_usd")).unwrap_or(Decimal::from(610));
    let gas_price_gwei = decimal_value(costs.get("gas_price_gwei")).unwrap_or(Decimal::from(3));
    let quote_gas = decimal_value(costs.get("quote_gas_units")).unwrap_or(Decimal::from(45_000));
    let swap_gas = decimal_value(costs.get("swap_gas_units")).unwrap_or(Decimal::from(210_000));
    let approval_gas =
        decimal_value(costs.get("approval_gas_units")).unwrap_or(Decimal::from(65_000));
    let approval_required = costs
        .get("approval_required")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let gas_units = quote_gas
        + swap_gas
        + if approval_required {
            approval_gas
        } else {
            Decimal::ZERO
        };
    let gas_usd = gas_units * gas_price_gwei / Decimal::from(1_000_000_000u64) * native_price;
    let slippage_pct = amount / Decimal::from(3_000_000) * Decimal::from(100) / Decimal::from(2)
        + Decimal::new(5, 2);
    let slippage_usd = amount * slippage_pct / Decimal::from(100);
    let cost_per_trade = gas_usd + slippage_usd;
    let daily_cost = cost_per_trade * daily_trade_target;
    let planned_cost = daily_cost * planned_days;
    let runway_days = if daily_cost > Decimal::ZERO {
        gas_float / daily_cost
    } else {
        Decimal::ZERO
    };
    let cost_bps = if amount > Decimal::ZERO {
        cost_per_trade / amount * Decimal::from(10_000)
    } else {
        Decimal::ZERO
    };
    let status = if nav_usd < min_nav || daily_cost > max_daily_cost || cost_bps > max_cost_bps {
        "blocking"
    } else if runway_days < planned_days {
        "watch"
    } else {
        "funded"
    };

    Ok(json!({
        "status": status,
        "config_path": BUDGET_CONFIG,
        "cost_config_path": cost_path,
        "report_path": report_path,
        "budget": {
            "name": config.get("name").cloned().unwrap_or(json!("Trading Budget")),
            "daily_trade_target": daily_trade_target.to_string(),
            "planned_competition_days": planned_days.to_string(),
            "operator_gas_float_usd": gas_float.round_dp(2).to_string(),
            "max_daily_execution_cost_usd": max_daily_cost.round_dp(2).to_string(),
            "max_cost_bps_per_trade": max_cost_bps.round_dp(2).to_string(),
            "min_nav_usd": min_nav.round_dp(2).to_string()
        },
        "current": {
            "nav_usd": nav_usd.round_dp(2).to_string(),
            "default_order_notional_usd": amount.round_dp(2).to_string(),
            "gas_usd_per_trade": gas_usd.round_dp(4).to_string(),
            "slippage_usd_per_trade": slippage_usd.round_dp(4).to_string(),
            "cost_usd_per_trade": cost_per_trade.round_dp(4).to_string(),
            "cost_bps_per_trade": cost_bps.round_dp(2).to_string(),
            "daily_execution_cost_usd": daily_cost.round_dp(4).to_string(),
            "planned_execution_cost_usd": planned_cost.round_dp(4).to_string(),
            "runway_days": runway_days.round_dp(2).to_string()
        }
    }))
}

fn decimal_value(value: Option<&Value>) -> Option<Decimal> {
    value
        .and_then(Value::as_f64)
        .and_then(Decimal::from_f64)
        .or_else(|| value.and_then(Value::as_i64).map(Decimal::from))
        .or_else(|| value.and_then(Value::as_u64).map(Decimal::from))
        .or_else(|| {
            value
                .and_then(Value::as_str)
                .and_then(|raw| raw.parse::<Decimal>().ok())
        })
}
