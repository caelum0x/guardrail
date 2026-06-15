//! Portfolio-management commands: rebalance previews, category exposure, the
//! operator playbook selector, the daily execution budget, mandate compilation,
//! and portfolio drift. These read the persisted run report plus policy configs
//! and never execute trades — they only preview intents.

use crate::util::{
    cost_bps, decimal_from_str, gas_cost_usd, json_decimal_field, json_decimal_or,
    scenario_shock_map,
};
use crate::{
    allocation_from_report, apply_preset, build_warmed_snapshot, category_map_from_universe,
    read_json_report, strategy_config, DEFAULT_UNIVERSE, SNAPSHOT_WARMUP_STEPS,
};
use common::decimal::to_f64;
use common::Settings;
use rust_decimal::Decimal;
use strategy_engine::StrategyEngine;

/// Preview the strategy's next target book and order intents.
pub fn run_rebalance(
    config: &str,
    report_path: &str,
    nav_override: Option<&str>,
    preset: &str,
) -> anyhow::Result<()> {
    let settings = Settings::load(config)?;
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let policy_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let policy = risk_engine::RiskPolicy::from_json_str(&policy_raw)?;
    let cap = (to_f64(policy.max_position_pct) - 1.0).max(1.0);
    let strat_cfg = apply_preset(strategy_config(&settings, cap), preset);

    let report = read_json_report(report_path).unwrap_or_else(|| serde_json::json!({}));
    let nav_usd = nav_override
        .and_then(decimal_from_str)
        .or_else(|| {
            report
                .get("nav_usd")
                .and_then(serde_json::Value::as_str)
                .and_then(decimal_from_str)
        })
        .unwrap_or_else(|| Decimal::from(10_000));
    let current = allocation_from_report(&report);
    let assets = universe.enabled_assets();
    let snapshot = build_warmed_snapshot(&assets, SNAPSHOT_WARMUP_STEPS);
    let decision = StrategyEngine::new(strat_cfg.clone()).decide(&snapshot, &current, nav_usd);

    println!("# Rebalance Preview");
    println!();
    println!("config: {config}");
    println!("report: {report_path}");
    println!("preset: {preset}");
    println!("nav_usd: {}", nav_usd.round_dp(2));
    println!("regime: {}", decision.regime.as_str());
    println!(
        "exposure multiplier: {:.2}",
        decision.regime.exposure_multiplier()
    );
    println!(
        "threshold: {:.2}% · max positions: {} · position cap: {:.2}%",
        strat_cfg.rebalance_threshold_pct,
        strat_cfg.max_positions,
        strat_cfg.max_position_weight_pct
    );
    println!();
    println!("{}", decision.explanation.headline);
    println!();
    println!("| Symbol | Current % | Target % | Delta % |");
    println!("|:-------|----------:|---------:|--------:|");
    for target in &decision.target_positions {
        let current_w = current.weight(&target.symbol);
        let delta = target.weight_pct - current_w;
        println!(
            "| {:<6} | {:>9.2} | {:>8.2} | {:>7.2} |",
            target.symbol, current_w, target.weight_pct, delta
        );
    }
    println!();
    if decision.proposed_orders.is_empty() {
        println!("No orders proposed; current allocation is within threshold.");
    } else {
        println!("| Side | From | To | Amount USD | Reason |");
        println!("|:-----|:-----|:---|-----------:|:-------|");
        for order in &decision.proposed_orders {
            println!(
                "| {:?} | {} | {} | {:>10.2} | {} |",
                order.side, order.from_symbol, order.to_symbol, order.amount_usd, order.reason
            );
        }
        println!();
        println!("preview_only: true; orders still require risk gate, TWAK quote, and execution.");
    }
    Ok(())
}

pub fn run_exposure(report_path: &str, universe_path: &str) -> anyhow::Result<()> {
    let report = read_json_report(report_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read report {report_path}"))?;
    let universe = read_json_report(universe_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read universe {universe_path}"))?;
    let categories = category_map_from_universe(&universe);
    let positions = report
        .get("positions")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut category_rows: std::collections::BTreeMap<String, (Decimal, Decimal, usize)> =
        std::collections::BTreeMap::new();
    let mut weights = Vec::new();
    let mut largest_symbol = String::from("-");
    let mut largest_weight = Decimal::ZERO;

    for position in &positions {
        let symbol = position
            .get("symbol")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("UNKNOWN");
        let value = json_decimal_field(position, "value_usd");
        let weight = json_decimal_field(position, "weight_pct");
        let category = categories
            .get(symbol)
            .cloned()
            .unwrap_or_else(|| "uncategorized".to_string());
        let entry = category_rows
            .entry(category)
            .or_insert((Decimal::ZERO, Decimal::ZERO, 0));
        entry.0 += value;
        entry.1 += weight;
        entry.2 += 1;
        weights.push(weight);
        if weight > largest_weight {
            largest_symbol = symbol.to_string();
            largest_weight = weight;
        }
    }

    weights.sort_by(|a, b| b.cmp(a));
    let top3: Decimal = weights.iter().take(3).copied().sum();
    let stable = category_rows
        .get("stable")
        .map(|(_, weight, _)| *weight)
        .unwrap_or(Decimal::ZERO);
    let total: Decimal = weights.iter().copied().sum();
    let risk = (total - stable).max(Decimal::ZERO);

    println!("# Exposure");
    println!();
    println!("report: {report_path}");
    println!("universe: {universe_path}");
    println!(
        "nav_usd: {}",
        report
            .get("nav_usd")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-")
    );
    println!("positions: {}", positions.len());
    println!("largest: {largest_symbol} {:.2}%", largest_weight);
    println!("top3_weight: {:.2}%", top3);
    println!("risk_weight: {:.2}% · stable_weight: {:.2}%", risk, stable);
    println!();
    println!("| Category | Positions | Weight % | Value USD |");
    println!("|:---------|----------:|---------:|----------:|");
    for (category, (value, weight, count)) in category_rows {
        println!(
            "| {:<14} | {:>9} | {:>8.2} | {:>9.2} |",
            category, count, weight, value
        );
    }
    Ok(())
}

pub fn run_playbook(report_path: &str, playbooks_path: &str) -> anyhow::Result<()> {
    let report = read_json_report(report_path);
    let playbooks = read_json_report(playbooks_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read playbooks {playbooks_path}"))?;
    let kill_switch = report
        .as_ref()
        .and_then(|value| value.get("kill_switch"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let active_id = if kill_switch {
        "kill_switch"
    } else if report.is_none() {
        "bootstrap"
    } else {
        "ready"
    };
    let active = playbooks
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("id").and_then(serde_json::Value::as_str) == Some(active_id))
        })
        .ok_or_else(|| anyhow::anyhow!("playbook '{active_id}' not found"))?;

    println!("# Operator Playbook");
    println!();
    println!("active_id: {active_id}");
    println!("report: {report_path}");
    println!("playbooks: {playbooks_path}");
    println!(
        "status: {}",
        active
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-")
    );
    println!(
        "label: {}",
        active
            .get("label")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-")
    );
    println!();
    if let Some(description) = active
        .get("description")
        .and_then(serde_json::Value::as_str)
    {
        println!("{description}");
        println!();
    }
    println!("commands:");
    if let Some(commands) = active.get("commands").and_then(serde_json::Value::as_array) {
        for command in commands {
            if let Some(command) = command.as_str() {
                println!("  {command}");
            }
        }
    }
    Ok(())
}

pub fn run_budget(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read budget config {config_path}"))?;
    let cost_path = config
        .get("cost_config_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("configs/costs/bsc_execution_costs.json");
    let report_path = config
        .get("report_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("data/run_report.json");
    let costs = read_json_report(cost_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read cost config {cost_path}"))?;
    let report = read_json_report(report_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read report {report_path}"))?;

    let nav = report
        .get("nav_usd")
        .and_then(serde_json::Value::as_str)
        .and_then(decimal_from_str)
        .unwrap_or(Decimal::ZERO);
    let daily_trades = json_decimal_or(&config, "daily_trade_target", Decimal::ONE);
    let planned_days = json_decimal_or(&config, "planned_competition_days", Decimal::from(7));
    let gas_float = json_decimal_or(&config, "operator_gas_float_usd", Decimal::ZERO);
    let max_daily = json_decimal_or(&config, "max_daily_execution_cost_usd", Decimal::from(5));
    let max_bps = json_decimal_or(&config, "max_cost_bps_per_trade", Decimal::from(25));
    let amount = json_decimal_or(&costs, "default_order_notional_usd", Decimal::from(1000));
    let gas_units = json_decimal_or(&costs, "quote_gas_units", Decimal::from(45_000))
        + json_decimal_or(&costs, "swap_gas_units", Decimal::from(210_000))
        + if costs
            .get("approval_required")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true)
        {
            json_decimal_or(&costs, "approval_gas_units", Decimal::from(65_000))
        } else {
            Decimal::ZERO
        };
    let gas_usd = gas_cost_usd(
        gas_units,
        json_decimal_or(&costs, "gas_price_gwei", Decimal::from(3)),
        json_decimal_or(&costs, "native_price_usd", Decimal::from(610)),
    );
    let slippage_pct = backtester::slippage::estimate_pct(amount, Decimal::from(3_000_000));
    let slippage_usd = amount * slippage_pct / Decimal::from(100);
    let per_trade = gas_usd + slippage_usd;
    let daily_cost = per_trade * daily_trades;
    let planned_cost = daily_cost * planned_days;
    let bps = cost_bps(per_trade, amount);
    let runway = if daily_cost > Decimal::ZERO {
        gas_float / daily_cost
    } else {
        Decimal::ZERO
    };
    let status = if daily_cost > max_daily || bps > max_bps {
        "blocking"
    } else if runway < planned_days {
        "watch"
    } else {
        "funded"
    };

    println!("# Trading Budget");
    println!();
    println!("status: {status}");
    println!("config: {config_path}");
    println!("report: {report_path}");
    println!("nav_usd: {:.2}", nav);
    println!("daily_trade_target: {:.2}", daily_trades);
    println!("runway_days: {:.2}", runway);
    println!("daily_cost_usd: {:.4} / max {:.2}", daily_cost, max_daily);
    println!("cost_bps: {:.2} / max {:.2}", bps, max_bps);
    println!("planned_cost_usd: {:.4}", planned_cost);
    Ok(())
}

pub fn run_mandates(config_path: &str) -> anyhow::Result<()> {
    let mandates = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read mandates config {config_path}"))?;
    println!("# Mandates");
    println!();
    println!("config: {config_path}");
    println!();
    println!("| ID | Hash | Max DD % | Position % | Reserve % | Slippage % |");
    println!("|:---|:-----|---------:|-----------:|----------:|-----------:|");
    if let Some(items) = mandates.as_array() {
        for item in items {
            let id = item
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            let mandate = item
                .get("mandate")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let compiled = policy_compiler::compile_mandate(mandate)?;
            println!(
                "| {} | {} | {:>8} | {:>10} | {:>9} | {:>10} |",
                id,
                compiled.hash,
                compiled.policy.max_total_drawdown_pct,
                compiled.policy.max_position_pct,
                compiled.policy.min_stable_reserve_pct,
                compiled.policy.max_slippage_pct
            );
        }
    }
    Ok(())
}

pub fn run_drift(policy_path: &str) -> anyhow::Result<()> {
    let policy = read_json_report(policy_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read drift policy {policy_path}"))?;
    let report_path = policy
        .get("report_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("data/run_report.json");
    let config_path = policy
        .get("config_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("configs/paper.toml");
    let warning = json_decimal_or(&policy, "warning_delta_pct", Decimal::from(3));
    let critical = json_decimal_or(&policy, "critical_delta_pct", Decimal::from(8));
    let max_turnover = json_decimal_or(&policy, "max_turnover_pct", Decimal::from(35));
    let stable_symbol = policy
        .get("stable_symbol")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("USDT");
    let report = read_json_report(report_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read report {report_path}"))?;
    let settings = Settings::load(config_path)?;
    let risk_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let risk_policy = risk_engine::RiskPolicy::from_json_str(&risk_raw)?;
    let cap = (to_f64(risk_policy.max_position_pct) - 1.0).max(1.0);
    let cfg = strategy_config(&settings, cap);
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let assets = universe.enabled_assets();
    let snapshot = build_warmed_snapshot(&assets, SNAPSHOT_WARMUP_STEPS);
    let nav = report
        .get("nav_usd")
        .and_then(serde_json::Value::as_str)
        .and_then(decimal_from_str)
        .unwrap_or_else(|| Decimal::from(10_000));
    let current = allocation_from_report(&report);
    let decision = StrategyEngine::new(cfg).decide(&snapshot, &current, nav);

    let mut rows = std::collections::BTreeMap::<String, (Decimal, Decimal)>::new();
    for (symbol, current_weight) in &current.weights_pct {
        rows.insert(symbol.clone(), (*current_weight, Decimal::ZERO));
    }
    for target in &decision.target_positions {
        rows.entry(target.symbol.clone())
            .and_modify(|entry| entry.1 = target.weight_pct)
            .or_insert((Decimal::ZERO, target.weight_pct));
    }

    let mut max_delta = Decimal::ZERO;
    let mut turnover = Decimal::ZERO;
    let mut ordered = Vec::new();
    for (symbol, (current_weight, target_weight)) in rows {
        let delta = target_weight - current_weight;
        let abs_delta = delta.abs();
        max_delta = max_delta.max(abs_delta);
        if symbol != stable_symbol {
            turnover += abs_delta;
        }
        ordered.push((symbol, current_weight, target_weight, delta, abs_delta));
    }
    ordered.sort_by(|a, b| b.4.cmp(&a.4));
    let status = if max_delta >= critical || turnover > max_turnover {
        "critical"
    } else if max_delta >= warning {
        "watch"
    } else {
        "aligned"
    };

    println!("# Portfolio Drift");
    println!();
    println!("status: {status}");
    println!("policy: {policy_path}");
    println!("report: {report_path}");
    println!("regime: {}", decision.regime.as_str());
    println!("max_delta_pct: {:.2}", max_delta);
    println!("turnover_pct: {:.2}", turnover);
    println!("turnover_usd: {:.2}", nav * turnover / Decimal::from(100));
    println!();
    println!("| Symbol | Current % | Target % | Delta % | Status |");
    println!("|:-------|----------:|---------:|--------:|:-------|");
    for (symbol, current_weight, target_weight, delta, abs_delta) in ordered {
        let row_status = if abs_delta >= critical {
            "critical"
        } else if abs_delta >= warning {
            "watch"
        } else {
            "normal"
        };
        println!(
            "| {:<6} | {:>9.2} | {:>8.2} | {:>7.2} | {} |",
            symbol, current_weight, target_weight, delta, row_status
        );
    }
    Ok(())
}

/// Apply configured market-stress scenarios to the current report's positions.
pub fn run_scenarios(
    report_path: &str,
    universe_path: &str,
    scenarios_path: &str,
) -> anyhow::Result<()> {
    let report = read_json_report(report_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read report {report_path}"))?;
    let universe = read_json_report(universe_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read universe {universe_path}"))?;
    let scenarios = read_json_report(scenarios_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read scenarios {scenarios_path}"))?;
    let categories = category_map_from_universe(&universe);
    let positions = report
        .get("positions")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let nav = report
        .get("nav_usd")
        .and_then(serde_json::Value::as_str)
        .and_then(decimal_from_str)
        .unwrap_or(Decimal::ZERO);

    println!("# Scenario Stress");
    println!();
    println!("report: {report_path}");
    println!("scenarios: {scenarios_path}");
    println!("nav_usd: {:.2}", nav);
    println!();
    println!("| Scenario | Return % | PnL USD | Largest Loss |");
    println!("|:---------|---------:|--------:|:-------------|");

    if let Some(items) = scenarios.as_array() {
        for scenario in items {
            let id = scenario
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            let shocks = scenario_shock_map(
                scenario
                    .get("category_shocks_pct")
                    .unwrap_or(&serde_json::Value::Object(Default::default())),
            );
            let mut pnl = Decimal::ZERO;
            let mut largest_symbol = String::from("-");
            let mut largest_loss = Decimal::ZERO;
            for position in &positions {
                let symbol = position
                    .get("symbol")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("UNKNOWN");
                let value = json_decimal_field(position, "value_usd");
                let category = categories
                    .get(symbol)
                    .map(String::as_str)
                    .unwrap_or("uncategorized");
                let shock = shocks
                    .get(category)
                    .copied()
                    .or_else(|| shocks.get("uncategorized").copied())
                    .unwrap_or(Decimal::ZERO);
                let position_pnl = (value * shock / Decimal::from(100)).round_dp(2);
                pnl += position_pnl;
                if position_pnl < largest_loss {
                    largest_loss = position_pnl;
                    largest_symbol = symbol.to_string();
                }
            }
            let ret = if nav > Decimal::ZERO {
                (pnl / nav * Decimal::from(100)).round_dp(2)
            } else {
                Decimal::ZERO
            };
            println!(
                "| {:<20} | {:>8.2} | {:>7.2} | {} {:.2} |",
                id, ret, pnl, largest_symbol, largest_loss
            );
        }
    }
    Ok(())
}
