//! Wallet-balance / stable-reserve drain protection.
//!
//! A buy order spends stable balance to acquire a risk asset. Even when the
//! resulting *position* is within limits, the trade must not drain the stable
//! reserve below the policy floor (`min_stable_reserve_pct`): the reserve is
//! what funds redemptions, gas, and orderly de-risking.
//!
//! The risk engine reasons over percentages of NAV. `RiskContext` exposes the
//! current `stable_reserve_pct` and `nav_usd`; the order carries `amount_usd`.
//! From these we can project the post-trade reserve without any new fields.

use crate::approval::RiskContext;
use crate::policy::RiskPolicy;
use common::{Decimal, OrderIntent, OrderSide};

/// True when a raw balance covers a raw requirement. Kept as a small, pure
/// predicate so it can be reused by callers that already hold absolute USD
/// figures rather than NAV percentages.
pub fn has_sufficient_balance(balance_usd: Decimal, required_usd: Decimal) -> bool {
    balance_usd >= required_usd
}

/// Project the stable reserve (as a percent of NAV) that would remain after a
/// buy of `amount_usd` settles. Returns `None` when NAV is non-positive (the
/// percentage is undefined and other checks own that failure mode).
fn projected_reserve_pct_after_buy(
    nav_usd: Decimal,
    current_reserve_pct: Decimal,
    amount_usd: Decimal,
) -> Option<Decimal> {
    if nav_usd <= Decimal::ZERO {
        return None;
    }
    let spent_pct = amount_usd / nav_usd * Decimal::from(100);
    Some(current_reserve_pct - spent_pct)
}

/// Pre-trade check: a buy must not push the stable reserve below the policy
/// floor. Sells add to the stable reserve (or are stable-neutral), so they are
/// never blocked here. Returns the existing `Vec<String>` reason shape.
pub fn check(policy: &RiskPolicy, intent: &OrderIntent, ctx: &RiskContext) -> Vec<String> {
    if intent.side != OrderSide::Buy {
        return Vec::new();
    }

    match projected_reserve_pct_after_buy(ctx.nav_usd, ctx.stable_reserve_pct, intent.amount_usd) {
        Some(projected) if projected < policy.min_stable_reserve_pct => vec![format!(
            "order would drain stable reserve to {projected}% (floor {}%)",
            policy.min_stable_reserve_pct
        )],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::OrderSide;
    use rust_decimal_macros::dec;

    fn ctx(nav: i64, reserve_pct: Decimal) -> RiskContext {
        let mut c = RiskContext::empty(Decimal::from(nav));
        c.stable_reserve_pct = reserve_pct;
        c
    }

    fn buy(amount: i64) -> OrderIntent {
        OrderIntent::new(OrderSide::Buy, "USDT", "CAKE", Decimal::from(amount), "t")
    }

    fn sell(amount: i64) -> OrderIntent {
        OrderIntent::new(OrderSide::Sell, "CAKE", "USDT", Decimal::from(amount), "t")
    }

    #[test]
    fn balance_predicate() {
        assert!(has_sufficient_balance(dec!(100), dec!(100)));
        assert!(has_sufficient_balance(dec!(101), dec!(100)));
        assert!(!has_sufficient_balance(dec!(99), dec!(100)));
    }

    #[test]
    fn buy_within_reserve_floor_passes() {
        // NAV 10k, reserve 30%. Spend 1k => 10% of NAV => reserve drops to 20%.
        let policy = RiskPolicy::default(); // floor 10%
        let reasons = check(&policy, &buy(1_000), &ctx(10_000, dec!(30)));
        assert!(reasons.is_empty(), "got {reasons:?}");
    }

    #[test]
    fn buy_draining_below_floor_is_rejected() {
        // NAV 10k, reserve 15%. Spend 1k => 10% => reserve 5% < 10% floor.
        let policy = RiskPolicy::default();
        let reasons = check(&policy, &buy(1_000), &ctx(10_000, dec!(15)));
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("stable reserve"), "got {reasons:?}");
    }

    #[test]
    fn buy_exactly_at_floor_passes() {
        // reserve 20%, spend 10% => 10% == floor, allowed (not below).
        let policy = RiskPolicy::default();
        let reasons = check(&policy, &buy(1_000), &ctx(10_000, dec!(20)));
        assert!(reasons.is_empty(), "got {reasons:?}");
    }

    #[test]
    fn sell_is_never_blocked_by_reserve() {
        let policy = RiskPolicy::default();
        // Even a huge sell against a tiny reserve must not be blocked here.
        let reasons = check(&policy, &sell(9_000), &ctx(10_000, dec!(1)));
        assert!(reasons.is_empty(), "got {reasons:?}");
    }

    #[test]
    fn non_positive_nav_defers_to_other_checks() {
        let policy = RiskPolicy::default();
        let reasons = check(&policy, &buy(1_000), &ctx(0, dec!(50)));
        assert!(reasons.is_empty(), "got {reasons:?}");
    }
}
