//! Volatility-target position sizing.
//!
//! Volatility targeting scales exposure so the position's expected volatility
//! matches a desired target. The leverage applied to capital is the ratio of
//! the target volatility to the asset's volatility:
//!
//! ```text
//! leverage = target_vol / asset_vol      (clamped to [0, max_leverage])
//! notional = leverage * capital
//! ```
//!
//! Both volatilities must be expressed on the same horizon (e.g. both
//! annualised, or both daily). The leverage is capped to avoid blowing up the
//! position when the asset is very quiet (`asset_vol` near zero).

use crate::error::{ensure_positive, Result};

/// Inputs for volatility-target sizing.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct VolTargetInput {
    /// Capital allocated to this position, in account currency. Must be > 0.
    pub capital: f64,
    /// Desired portfolio/position volatility (same horizon as `asset_vol`).
    /// Must be > 0 (e.g. `0.10` for a 10% annualised target).
    pub target_vol: f64,
    /// Realised/forecast volatility of the asset (same horizon as
    /// `target_vol`). Must be > 0 (e.g. `0.20` for 20% annualised).
    pub asset_vol: f64,
    /// Maximum allowed leverage. The computed leverage is clamped to
    /// `[0, max_leverage]`. Must be > 0 (e.g. `1.0` for no leverage, `3.0` to
    /// allow up to 3x).
    pub max_leverage: f64,
}

/// Output of volatility-target sizing.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VolTargetOutput {
    /// Leverage actually applied after capping (`min(target/asset, max)`).
    pub leverage: f64,
    /// Position notional value (`leverage * capital`).
    pub notional: f64,
    /// `true` if the raw leverage was clamped by `max_leverage`.
    pub capped: bool,
}

/// Compute a volatility-target position size.
///
/// `leverage = clamp(target_vol / asset_vol, 0, max_leverage)` and
/// `notional = leverage * capital`. Reports whether the cap bound.
///
/// # Examples
/// ```
/// use position_sizer::vol_target::{vol_target, VolTargetInput};
/// // 10% target vol, 20% asset vol -> 0.5x leverage on $100k -> $50k notional.
/// let out = vol_target(VolTargetInput {
///     capital: 100_000.0,
///     target_vol: 0.10,
///     asset_vol: 0.20,
///     max_leverage: 3.0,
/// }).unwrap();
/// assert_eq!(out.leverage, 0.5);
/// assert_eq!(out.notional, 50_000.0);
/// assert!(!out.capped);
/// ```
pub fn vol_target(input: VolTargetInput) -> Result<VolTargetOutput> {
    ensure_positive("capital", input.capital)?;
    ensure_positive("target_vol", input.target_vol)?;
    ensure_positive("asset_vol", input.asset_vol)?;
    ensure_positive("max_leverage", input.max_leverage)?;

    let raw_leverage = input.target_vol / input.asset_vol;
    let capped = raw_leverage > input.max_leverage;
    let leverage = if capped {
        input.max_leverage
    } else {
        raw_leverage
    };
    let notional = leverage * input.capital;

    Ok(VolTargetOutput {
        leverage,
        notional,
        capped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SizingError;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "expected {b}, got {a}");
    }

    #[test]
    fn known_value_half_leverage() {
        let out = vol_target(VolTargetInput {
            capital: 100_000.0,
            target_vol: 0.10,
            asset_vol: 0.20,
            max_leverage: 3.0,
        })
        .unwrap();
        approx(out.leverage, 0.5);
        approx(out.notional, 50_000.0);
        assert!(!out.capped);
    }

    #[test]
    fn quiet_asset_triggers_cap() {
        // target 10%, asset 2% -> raw leverage 5x, capped to 3x.
        let out = vol_target(VolTargetInput {
            capital: 100_000.0,
            target_vol: 0.10,
            asset_vol: 0.02,
            max_leverage: 3.0,
        })
        .unwrap();
        approx(out.leverage, 3.0);
        approx(out.notional, 300_000.0);
        assert!(out.capped);
    }

    #[test]
    fn equal_vols_give_unit_leverage() {
        let out = vol_target(VolTargetInput {
            capital: 250_000.0,
            target_vol: 0.15,
            asset_vol: 0.15,
            max_leverage: 2.0,
        })
        .unwrap();
        approx(out.leverage, 1.0);
        approx(out.notional, 250_000.0);
        assert!(!out.capped);
    }

    #[test]
    fn cap_exactly_at_boundary_is_not_capped() {
        // raw leverage == max_leverage; clamp keeps the value, capped=false.
        let out = vol_target(VolTargetInput {
            capital: 100_000.0,
            target_vol: 0.30,
            asset_vol: 0.10,
            max_leverage: 3.0,
        })
        .unwrap();
        approx(out.leverage, 3.0);
        assert!(!out.capped);
    }

    #[test]
    fn rejects_zero_asset_vol() {
        assert!(matches!(
            vol_target(VolTargetInput {
                capital: 100_000.0,
                target_vol: 0.10,
                asset_vol: 0.0,
                max_leverage: 3.0,
            }),
            Err(SizingError::NotPositive { .. })
        ));
    }
}
