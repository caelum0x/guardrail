//! Regime-aware ensemble meta-allocator for Guardrail Alpha.
//!
//! This crate sits *above* the Track-2 strategy skills. For a single classified
//! [`MarketRegime`](strategy_engine::MarketRegime) it blends each skill's proposed
//! target book into one **blended target portfolio** — a weighted average of every
//! skill's per-symbol target weight, renormalized so the risk allocation is
//! `<= max_risk_allocation_pct` with the remainder held as a USDT reserve — plus a
//! **per-skill contribution attribution**.
//!
//! The blend is *advisory only*: the [`risk_engine`] remains the sole execution
//! gate (per-position caps, stable-reserve floor, drawdown kill-switch). This crate
//! executes nothing; it only proposes a target book for the engine to validate. It
//! mirrors `python-lab/guardrail_lab/ensemble.py` so the Rust and Python paths
//! agree conceptually.
//!
//! # Design contract
//!
//! * **Pure + typed.** [`blend_targets`] reads no I/O and returns frozen values.
//! * **Never panics on empty input.** Missing weights, an unknown regime, or no
//!   skill books yield a typed [`EnsembleResult`] with `ok == false` and a
//!   human-readable `reason` — never a panic.
//! * **Offline-safe.** The `skills/ensemble.json` weight table is embedded at
//!   compile time via [`EnsembleConfig::embedded`]; runtime parsing from a path or
//!   string is also available.
//!
//! # Example
//!
//! ```
//! use strategy_ensemble::{blend_targets, EnsembleConfig, SkillTargets};
//! use strategy_engine::MarketRegime;
//! use common::TargetPosition;
//! use rust_decimal::Decimal;
//!
//! let config = EnsembleConfig::embedded().expect("embedded config parses");
//! let books = vec![SkillTargets::new(
//!     "trend-breakout-momentum",
//!     vec![TargetPosition { symbol: "BTC".into(), weight_pct: Decimal::from(50) }],
//! )];
//! let result = blend_targets(&config, MarketRegime::Breakout, &books);
//! assert!(result.ok);
//! ```

pub mod blend;
pub mod error;
pub mod weights;

pub use blend::{blend_targets, EnsembleResult, SkillContribution, SkillTargets};
pub use error::EnsembleError;
pub use weights::{
    EnsembleConfig, RegimeWeights, DEFAULT_MAX_RISK_ALLOCATION_PCT, DEFAULT_RESERVE_SYMBOL,
    EMBEDDED_CONFIG,
};

// Re-export the regime type so callers don't need a direct strategy-engine dep.
pub use strategy_engine::MarketRegime;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_config_parses_and_has_all_regimes() {
        let cfg = EnsembleConfig::embedded().expect("embedded ensemble.json parses");
        assert_eq!(cfg.reserve_symbol, "USDT");
        for regime in [
            MarketRegime::RiskOn,
            MarketRegime::RiskOff,
            MarketRegime::Chop,
            MarketRegime::Breakout,
        ] {
            let rw = cfg.regime(regime).expect("regime present in config");
            let norm = rw.normalized();
            let sum: f64 = norm.values().sum();
            assert!(
                (sum - 1.0).abs() < 1e-9,
                "regime {} weights renormalize to 1.0",
                regime.as_str()
            );
        }
    }

    #[test]
    fn embedded_blend_produces_book_summing_to_max_risk() {
        use common::TargetPosition;
        use rust_decimal::Decimal;

        let cfg = EnsembleConfig::embedded().unwrap();
        let books = vec![
            SkillTargets::new(
                "trend-breakout-momentum",
                vec![TargetPosition {
                    symbol: "BTC".into(),
                    weight_pct: Decimal::from(40),
                }],
            ),
            SkillTargets::new(
                "cmc-regime-routed-alpha",
                vec![TargetPosition {
                    symbol: "ETH".into(),
                    weight_pct: Decimal::from(30),
                }],
            ),
        ];
        let result = blend_targets(&cfg, MarketRegime::Breakout, &books);
        assert!(result.ok);
        let total: Decimal = result.blended.iter().map(|p| p.weight_pct).sum();
        assert_eq!(total, Decimal::from(100));
    }
}
