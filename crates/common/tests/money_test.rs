//! Tests for the Money value type.

use common::money::Money;
use common::Decimal;

#[test]
fn usd_constructs_with_usd_currency() {
    let m = Money::usd(Decimal::from(100));
    assert_eq!(m.amount, Decimal::from(100));
    assert_eq!(m.currency, "USD");
}

#[test]
fn zero_usd_is_zero_amount_in_usd() {
    let m = Money::zero_usd();
    assert_eq!(m.amount, Decimal::ZERO);
    assert_eq!(m.currency, "USD");
}

#[test]
fn display_renders_amount_and_currency() {
    let m = Money::usd(Decimal::new(1250, 2)); // 12.50
    assert_eq!(m.to_string(), "12.50 USD");
}

#[test]
fn money_serde_round_trip() {
    let m = Money::usd(Decimal::from(42));
    let json = serde_json::to_string(&m).unwrap();
    let back: Money = serde_json::from_str(&json).unwrap();
    assert_eq!(back, m);
}
