//! # position-sizer
//!
//! Pure-Rust position sizing algorithms with no external service dependencies.
//! Every routine validates its inputs at the boundary and returns a typed
//! [`SizingError`](error::SizingError) instead of panicking or emitting
//! `NaN`/`Inf`.
//!
//! ## Algorithms
//!
//! - [`fixed_fractional`] — risk a constant fraction of equity per trade,
//!   converting a stop distance into a unit count and notional.
//! - [`vol_target`] — scale exposure so position volatility matches a target
//!   (`leverage = target_vol / asset_vol`, capped by `max_leverage`).
//! - [`kelly`] — Kelly-optimal stake fraction `f* = edge / odds`, with a
//!   fractional-Kelly multiplier and a hard cap.
//! - [`equal_risk`] — inverse-volatility (equal-risk-contribution) portfolio
//!   weights for a set of assets given their volatilities.
//! - [`decimal`] — exact `rust_decimal` fixed-fractional sizing with lot
//!   rounding for order-quantity conversion.
//!
//! All public input/output structs derive `serde::Serialize`/`Deserialize`
//! so they can cross process and API boundaries.
//!
//! ## Example
//!
//! ```
//! use position_sizer::vol_target::{vol_target, VolTargetInput};
//!
//! let out = vol_target(VolTargetInput {
//!     capital: 1_000_000.0,
//!     target_vol: 0.10,
//!     asset_vol: 0.25,
//!     max_leverage: 2.0,
//! })
//! .unwrap();
//! assert!((out.leverage - 0.4).abs() < 1e-12);
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod decimal;
pub mod equal_risk;
pub mod error;
pub mod fixed_fractional;
pub mod kelly;
pub mod vol_target;

pub use equal_risk::{equal_risk_contribution, AssetWeight};
pub use error::{Result, SizingError};
pub use fixed_fractional::{fixed_fractional, FixedFractionalInput, FixedFractionalOutput};
pub use kelly::{kelly_even_money, kelly_fraction, KellyInput, KellyOutput};
pub use vol_target::{vol_target, VolTargetInput, VolTargetOutput};

#[cfg(test)]
mod integration_tests {
    //! Cross-module sanity checks combining several sizing routines.
    use super::*;

    #[test]
    fn full_pipeline_vol_target_then_kelly_scaled() {
        // Allocate capital by ERC, then size the largest sleeve with vol target.
        let weights = equal_risk_contribution(&[
            ("BTC".to_string(), 0.50),
            ("ETH".to_string(), 0.60),
            ("SOL".to_string(), 0.90),
        ])
        .unwrap();
        let total: f64 = weights.iter().map(|w| w.weight).sum();
        assert!((total - 1.0).abs() < 1e-12);

        // Lowest-vol asset (BTC) should get the largest weight.
        assert!(weights[0].weight > weights[1].weight);
        assert!(weights[1].weight > weights[2].weight);

        // Size BTC sleeve at 20% target vol against its 50% realised vol.
        let btc_capital = 1_000_000.0 * weights[0].weight;
        let sized = vol_target(VolTargetInput {
            capital: btc_capital,
            target_vol: 0.20,
            asset_vol: 0.50,
            max_leverage: 1.0,
        })
        .unwrap();
        assert!((sized.leverage - 0.4).abs() < 1e-12);
        assert!((sized.notional - btc_capital * 0.4).abs() < 1e-6);
    }

    #[test]
    fn kelly_then_fixed_fractional_consistency() {
        // Use Kelly to derive a risk fraction, feed it to fixed-fractional.
        let k = kelly_even_money(0.55, 0.5, 0.1).unwrap();
        // f* = 2*0.55-1 = 0.1; half-Kelly -> 0.05.
        assert!((k.fraction_of_capital - 0.05).abs() < 1e-12);

        let pos = fixed_fractional(FixedFractionalInput {
            equity: 200_000.0,
            risk_fraction: k.fraction_of_capital,
            entry_price: 100.0,
            risk_per_unit: 5.0,
        })
        .unwrap();
        // 200k * 5% = 10k risk; /5 = 2000 units; *100 = 200k notional.
        assert!((pos.units - 2_000.0).abs() < 1e-9);
        assert!((pos.notional - 200_000.0).abs() < 1e-6);
    }
}
