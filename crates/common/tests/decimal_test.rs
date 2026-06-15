//! Tests for the shared decimal helpers.

use common::decimal::{apply_pct, clamp, pct_to_fraction, to_f64};
use common::Decimal;

#[test]
fn pct_to_fraction_divides_by_hundred() {
    assert_eq!(pct_to_fraction(Decimal::from(7)), Decimal::new(7, 2));
}

#[test]
fn apply_pct_applies_percentage_to_base() {
    // 10% of 250 = 25
    assert_eq!(
        apply_pct(Decimal::from(250), Decimal::from(10)),
        Decimal::from(25)
    );
}

#[test]
fn to_f64_converts_lossily() {
    assert_eq!(to_f64(Decimal::new(150, 2)), 1.5);
}

#[test]
fn clamp_bounds_value_into_range() {
    let lo = Decimal::from(0);
    let hi = Decimal::from(10);
    assert_eq!(clamp(Decimal::from(-5), lo, hi), lo);
    assert_eq!(clamp(Decimal::from(15), lo, hi), hi);
    assert_eq!(clamp(Decimal::from(5), lo, hi), Decimal::from(5));
}
