//! Helpers for converting `Decimal` price series into `f64` slices.
//!
//! The core indicator functions operate on `&[f64]` to stay dependency-light
//! and fast. Callers holding `rust_decimal::Decimal` candle data (such as
//! `cmc-client`'s `Candle`) can use these helpers to convert before calling
//! the indicators.

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/// Convert a slice of `Decimal` values into a `Vec<f64>`.
///
/// Any value that cannot be represented as `f64` is replaced with `0.0`.
/// `Decimal` -> `f64` only fails for non-finite internal states which cannot
/// occur for a well-formed `Decimal`, so this is effectively lossless for
/// realistic price data.
#[must_use]
pub fn decimals_to_f64(values: &[Decimal]) -> Vec<f64> {
    values.iter().map(|d| d.to_f64().unwrap_or(0.0)).collect()
}

/// Convert a single `Decimal` into an `f64`, returning `None` if conversion
/// is not possible.
#[must_use]
pub fn decimal_to_f64(value: Decimal) -> Option<f64> {
    value.to_f64()
}
