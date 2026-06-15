//! Decimal helpers. We use `rust_decimal::Decimal` everywhere money or
//! percentages are involved to avoid binary floating-point drift.

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/// Convert a percentage (e.g. `7` for 7%) into a multiplier fraction (`0.07`).
pub fn pct_to_fraction(pct: Decimal) -> Decimal {
    pct / Decimal::from(100)
}

/// Apply `pct` percent to `base`.
pub fn apply_pct(base: Decimal, pct: Decimal) -> Decimal {
    base * pct_to_fraction(pct)
}

/// Lossy conversion to f64 for charting / scoring math. Never used for money.
pub fn to_f64(d: Decimal) -> f64 {
    d.to_f64().unwrap_or(0.0)
}

/// Clamp a decimal into an inclusive range.
pub fn clamp(value: Decimal, min: Decimal, max: Decimal) -> Decimal {
    value.max(min).min(max)
}
