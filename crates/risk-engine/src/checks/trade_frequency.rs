//! Trade-frequency / heartbeat-sizing gate.
//!
//! The policy's [`DailyTradeRequirement`](crate::policy::DailyTradeRequirement)
//! is the only trade-cadence data the engine owns. It expresses:
//!
//! * `min_trades_per_day` — the activity floor the strategy must meet, and
//! * `max_heartbeat_trade_pct` — the largest size (as % of NAV) a *heartbeat*
//!   trade may take. Heartbeats are the tiny keep-alive trades the strategy
//!   fires to satisfy the activity floor; they must stay small so they cannot
//!   be abused as a back door for full-size trades.
//!
//! The engine does not track a rolling trade timestamp window (no such field
//! exists on the public structs and we must not add one), so a true
//! per-second/minute rate limit lives in the throttle layer, not here. What we
//! *can* enforce honestly from the data on hand is the heartbeat size cap: a
//! trade tagged as a heartbeat must not exceed `max_heartbeat_trade_pct` of NAV.
//!
//! [`daily_requirement_enabled`] remains the public predicate other modules use
//! to gate cadence logic; [`check_heartbeat_size`] is the real enforcement.

use crate::approval::RiskContext;
use crate::policy::RiskPolicy;
use common::{Decimal, OrderIntent};

/// Reason tag a strategy uses to mark an order as a low-stakes heartbeat trade.
/// Matched as a case-insensitive substring of `OrderIntent.reason`.
const HEARTBEAT_REASON_TAG: &str = "heartbeat";

/// True when the policy requires a minimum daily trade cadence.
pub fn daily_requirement_enabled(policy: &RiskPolicy) -> bool {
    policy.daily_trade_requirement.enabled
}

/// True when an order's `reason` marks it as a heartbeat (keep-alive) trade.
fn is_heartbeat(intent: &OrderIntent) -> bool {
    intent
        .reason
        .to_lowercase()
        .contains(HEARTBEAT_REASON_TAG)
}

/// Enforce the heartbeat size cap. A heartbeat trade larger than
/// `max_heartbeat_trade_pct` of NAV is rejected so the cadence requirement can
/// never be used to push oversized orders. Non-heartbeat orders and disabled
/// requirements are no-ops. NAV must be positive for the percentage to be
/// meaningful; otherwise we defer to the position/notional checks.
pub fn check(policy: &RiskPolicy, intent: &OrderIntent, ctx: &RiskContext) -> Vec<String> {
    let req = &policy.daily_trade_requirement;
    if !req.enabled || !is_heartbeat(intent) || ctx.nav_usd <= Decimal::ZERO {
        return Vec::new();
    }

    let max_heartbeat_usd = ctx.nav_usd * req.max_heartbeat_trade_pct / Decimal::from(100);
    if intent.amount_usd > max_heartbeat_usd {
        vec![format!(
            "heartbeat trade {} USD exceeds max heartbeat size {}% of NAV ({} USD)",
            intent.amount_usd, req.max_heartbeat_trade_pct, max_heartbeat_usd
        )]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::DailyTradeRequirement;
    use common::OrderSide;
    use rust_decimal_macros::dec;

    fn policy_with(req: DailyTradeRequirement) -> RiskPolicy {
        RiskPolicy {
            daily_trade_requirement: req,
            ..RiskPolicy::default()
        }
    }

    fn order(amount: i64, reason: &str) -> OrderIntent {
        OrderIntent::new(OrderSide::Buy, "USDT", "CAKE", Decimal::from(amount), reason)
    }

    fn ctx() -> RiskContext {
        RiskContext::empty(Decimal::from(10_000))
    }

    #[test]
    fn enabled_reflects_policy() {
        let on = DailyTradeRequirement {
            enabled: true,
            ..DailyTradeRequirement::default()
        };
        assert!(daily_requirement_enabled(&policy_with(on)));
        let off = DailyTradeRequirement {
            enabled: false,
            ..DailyTradeRequirement::default()
        };
        assert!(!daily_requirement_enabled(&policy_with(off)));
    }

    #[test]
    fn non_heartbeat_order_is_ignored() {
        // Huge normal order is not constrained by the heartbeat cap.
        let policy = RiskPolicy::default(); // 2% heartbeat cap => 200 USD on 10k
        let reasons = check(&policy, &order(5_000, "rebalance"), &ctx());
        assert!(reasons.is_empty(), "got {reasons:?}");
    }

    #[test]
    fn small_heartbeat_passes() {
        let policy = RiskPolicy::default(); // 2% of 10k = 200 USD cap
        let reasons = check(&policy, &order(150, "heartbeat keepalive"), &ctx());
        assert!(reasons.is_empty(), "got {reasons:?}");
    }

    #[test]
    fn heartbeat_at_cap_passes() {
        let policy = RiskPolicy::default();
        let reasons = check(&policy, &order(200, "heartbeat"), &ctx());
        assert!(reasons.is_empty(), "got {reasons:?}");
    }

    #[test]
    fn oversized_heartbeat_is_rejected() {
        let policy = RiskPolicy::default();
        let reasons = check(&policy, &order(500, "daily heartbeat"), &ctx());
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("heartbeat"), "got {reasons:?}");
    }

    #[test]
    fn disabled_requirement_is_noop() {
        let req = DailyTradeRequirement {
            enabled: false,
            max_heartbeat_trade_pct: dec!(2),
            ..DailyTradeRequirement::default()
        };
        let reasons = check(&policy_with(req), &order(5_000, "heartbeat"), &ctx());
        assert!(reasons.is_empty(), "got {reasons:?}");
    }

    #[test]
    fn non_positive_nav_is_noop() {
        let policy = RiskPolicy::default();
        let reasons = check(&policy, &order(5_000, "heartbeat"), &RiskContext::empty(Decimal::ZERO));
        assert!(reasons.is_empty(), "got {reasons:?}");
    }
}
