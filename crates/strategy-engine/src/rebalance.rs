//! Rebalancing: convert target weights into concrete order intents, gated by a
//! no-churn threshold. All risk-asset trades route through the stable reserve.

use crate::strategy_config::StrategyConfig;
use crate::target_portfolio::CurrentAllocation;
use common::constants::RESERVE_SYMBOL;
use common::decimal::{apply_pct, to_f64};
use common::{Decimal, OrderIntent, OrderSide, TargetPosition};
use rust_decimal::prelude::FromPrimitive;

/// Compute the orders that move `current` toward `targets`.
pub fn compute_orders(
    targets: &[TargetPosition],
    current: &CurrentAllocation,
    nav_usd: Decimal,
    cfg: &StrategyConfig,
) -> Vec<OrderIntent> {
    let threshold = Decimal::from_f64(cfg.rebalance_threshold_pct).unwrap_or(Decimal::ZERO);
    let mut orders = Vec::new();

    // 1. Adjust toward each non-reserve target.
    for t in targets.iter().filter(|t| t.symbol != RESERVE_SYMBOL) {
        let current_w = current.weight(&t.symbol);
        let delta = t.weight_pct - current_w;
        if delta.abs() < threshold {
            continue;
        }
        let amount = apply_pct(nav_usd, delta.abs());
        if delta > Decimal::ZERO {
            orders.push(OrderIntent::new(
                OrderSide::Buy,
                RESERVE_SYMBOL,
                &t.symbol,
                amount,
                format!("increase {} toward target {}%", t.symbol, t.weight_pct),
            ));
        } else {
            orders.push(OrderIntent::new(
                OrderSide::Sell,
                &t.symbol,
                RESERVE_SYMBOL,
                amount,
                format!("trim {} toward target {}%", t.symbol, t.weight_pct),
            ));
        }
    }

    // 2. Exit anything held that is no longer a target.
    for symbol in current.held_symbols() {
        if symbol == RESERVE_SYMBOL {
            continue;
        }
        let still_target = targets.iter().any(|t| t.symbol == symbol);
        if still_target {
            continue;
        }
        let current_w = current.weight(&symbol);
        if to_f64(current_w) <= 0.0 {
            continue;
        }
        let amount = apply_pct(nav_usd, current_w);
        orders.push(OrderIntent::new(
            OrderSide::Sell,
            &symbol,
            RESERVE_SYMBOL,
            amount,
            format!("exit {} (dropped from target set)", symbol),
        ));
    }

    orders
}
