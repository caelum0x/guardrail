//! Daily-trade requirement. Track 1 requires a minimum of one trade per UTC
//! day. When the strategy would otherwise sit idle, this produces a tiny
//! compliant "heartbeat" trade that still respects risk limits.

use common::constants::RESERVE_SYMBOL;
use common::decimal::apply_pct;
use common::time::utc_day;
use common::{Decimal, OrderIntent, OrderSide};

/// Has at least one trade been recorded for the current UTC day?
pub fn satisfied_today(last_trade_ms: Option<i64>, now_ms: i64) -> bool {
    match last_trade_ms {
        Some(ts) => utc_day(ts) == utc_day(now_ms),
        None => false,
    }
}

/// Build a minimal heartbeat order: a small round-trip into the top symbol,
/// sized at `heartbeat_pct` percent of NAV, used only to satisfy the daily
/// requirement when no real rebalance fired.
pub fn heartbeat_order(
    top_symbol: &str,
    nav_usd: Decimal,
    heartbeat_pct: Decimal,
) -> Option<OrderIntent> {
    if top_symbol == RESERVE_SYMBOL || nav_usd <= Decimal::ZERO {
        return None;
    }
    let amount = apply_pct(nav_usd, heartbeat_pct);
    Some(OrderIntent::new(
        OrderSide::Buy,
        RESERVE_SYMBOL,
        top_symbol,
        amount,
        "daily-trade heartbeat".to_string(),
    ))
}
