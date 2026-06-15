//! Tests for the daily-trade requirement: same-UTC-day detection and the
//! heartbeat order builder that keeps the strategy compliant on idle days.

use common::constants::RESERVE_SYMBOL;
use common::{Decimal, OrderSide};
use strategy_engine::daily_trade::{heartbeat_order, satisfied_today};

const MS_PER_DAY: i64 = 24 * 60 * 60 * 1000;

#[test]
fn satisfied_today_false_when_never_traded() {
    assert!(!satisfied_today(None, MS_PER_DAY * 100));
}

#[test]
fn satisfied_today_true_for_same_utc_day() {
    // Two timestamps within the same UTC calendar day.
    let morning = MS_PER_DAY * 100 + 1_000;
    let evening = MS_PER_DAY * 100 + 20 * 60 * 60 * 1000;
    assert!(satisfied_today(Some(morning), evening));
}

#[test]
fn satisfied_today_false_for_different_utc_day() {
    let yesterday = MS_PER_DAY * 100;
    let today = MS_PER_DAY * 101;
    assert!(!satisfied_today(Some(yesterday), today));
}

#[test]
fn heartbeat_none_for_reserve_symbol() {
    let order = heartbeat_order(RESERVE_SYMBOL, Decimal::from(1000), Decimal::from(1));
    assert!(order.is_none());
}

#[test]
fn heartbeat_none_when_nav_is_zero() {
    let order = heartbeat_order("AAA", Decimal::ZERO, Decimal::from(1));
    assert!(order.is_none());
}

#[test]
fn heartbeat_none_when_nav_is_negative() {
    let order = heartbeat_order("AAA", Decimal::from(-50), Decimal::from(1));
    assert!(order.is_none());
}

#[test]
fn heartbeat_sized_at_pct_for_risk_symbol() {
    // 0.5% of NAV 2000 = 10 USD, routed from the reserve into the top symbol.
    let order = heartbeat_order(
        "AAA",
        Decimal::from(2000),
        Decimal::from_f64_retain(0.5).unwrap(),
    )
    .expect("expected a heartbeat order for a risk symbol");
    assert_eq!(order.side, OrderSide::Buy);
    assert_eq!(order.from_symbol, RESERVE_SYMBOL);
    assert_eq!(order.to_symbol, "AAA");
    assert_eq!(order.amount_usd, Decimal::from(10));
}
