//! The weighted-average blend math.
//!
//! Given a classified [`MarketRegime`], its per-skill blend weights (from the
//! [`EnsembleConfig`](crate::EnsembleConfig)), and each skill's proposed target
//! book, [`blend_targets`] produces the blended target book plus a per-skill
//! contribution attribution.
//!
//! This mirrors `python-lab/guardrail_lab/ensemble.py::blend_regime` /
//! `_finalize_book` so the Rust and Python paths agree conceptually:
//!
//! * for each symbol, the blended **risk** weight is
//!   `Σ blend_weight[skill] * skill_weight_pct[symbol]` over non-reserve
//!   positions;
//! * the summed risk book is renormalized to `<= max_risk_allocation_pct`
//!   (scaled down proportionally only when it would over-allocate);
//! * the remainder `max_risk - Σ risk` is held as a single USDT reserve line.
//!
//! All math uses [`Decimal`] to avoid binary floating-point drift, matching the
//! rest of the workspace. The function is pure and never panics: empty inputs
//! yield an empty, typed [`EnsembleResult`] carrying a human-readable `reason`.

use crate::weights::EnsembleConfig;
use common::{Decimal, TargetPosition};
use rust_decimal::prelude::FromPrimitive;
use std::collections::BTreeMap;
use strategy_engine::MarketRegime;

/// A single skill's proposed target book for one regime.
///
/// The `skill` name must match a key in the config's per-regime `weights` map
/// for the skill to receive any blend weight.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillTargets {
    /// The skill directory name (e.g. `"mean-reversion-chop"`).
    pub skill: String,
    /// The skill's own proposed target book for this regime.
    pub targets: Vec<TargetPosition>,
}

impl SkillTargets {
    /// Convenience constructor.
    pub fn new(skill: impl Into<String>, targets: Vec<TargetPosition>) -> Self {
        SkillTargets {
            skill: skill.into(),
            targets,
        }
    }
}

/// How a single skill contributed to the blended book for one regime.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillContribution {
    /// The skill directory name.
    pub skill: String,
    /// The skill's normalized blend weight for this regime, in `[0, 1]`.
    pub blend_weight: Decimal,
    /// Total *risk* weight (excluding the reserve symbol) in the skill's own
    /// book, in percentage points.
    pub risk_weight_pct: Decimal,
    /// `blend_weight * risk_weight_pct` — the skill's pre-renormalization
    /// contribution to the blended risk book.
    pub contributed_pct: Decimal,
    /// `true` when the skill supplied a non-empty book that was blended in.
    pub loaded: bool,
    /// Human-readable note (empty when `loaded` is `true`).
    pub reason: String,
}

/// The blended target book and attribution for a single regime.
#[derive(Debug, Clone, PartialEq)]
pub struct EnsembleResult {
    /// The regime this blend was computed for.
    pub regime: MarketRegime,
    /// `true` when at least one skill book was blended into a non-empty result.
    pub ok: bool,
    /// Human-readable explanation (empty on clean success; populated on failure
    /// or when some skills were skipped).
    pub reason: String,
    /// The regime's rationale text copied from the config.
    pub rationale: String,
    /// The blended target book: risk positions first (descending weight, then
    /// symbol), with the reserve symbol last. Renormalized to
    /// `<= max_risk_allocation_pct`.
    pub blended: Vec<TargetPosition>,
    /// The symbol holding the unallocated remainder.
    pub reserve_symbol: String,
    /// The reserve weight in percentage points.
    pub reserve_pct: Decimal,
    /// Per-skill contribution attribution for this regime.
    pub attribution: Vec<SkillContribution>,
}

impl EnsembleResult {
    /// An empty, failed result carrying `reason` and the regime.
    fn empty(regime: MarketRegime, reserve_symbol: &str, reason: impl Into<String>) -> Self {
        EnsembleResult {
            regime,
            ok: false,
            reason: reason.into(),
            rationale: String::new(),
            blended: Vec::new(),
            reserve_symbol: reserve_symbol.to_string(),
            reserve_pct: Decimal::ZERO,
            attribution: Vec::new(),
        }
    }
}

/// Round a [`Decimal`] to `dp` decimal places (banker's-rounding-free, midpoint
/// away from zero — matching Python's `round` closely enough for weights).
fn round_dp(value: Decimal, dp: u32) -> Decimal {
    value.round_dp(dp)
}

/// Compute the weighted-average blended target book for `regime`.
///
/// `per_skill_targets` carries each skill's own proposed book; the blend weight
/// for each skill is taken from `config` for this `regime` (renormalized to sum
/// to 1.0). Skills present in `per_skill_targets` but not in the config's
/// per-regime weights receive zero weight and are reported as not loaded.
///
/// Never panics. On any failure (regime not configured, no valid weights, no
/// non-empty skill books) `ok` is `false` and `reason` explains why.
pub fn blend_targets(
    config: &EnsembleConfig,
    regime: MarketRegime,
    per_skill_targets: &[SkillTargets],
) -> EnsembleResult {
    let reserve_symbol = config.reserve_symbol.clone();

    let regime_cfg = match config.regime(regime) {
        Some(cfg) => cfg,
        None => {
            let available = config
                .regimes
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            let available = if available.is_empty() {
                "(none)".to_string()
            } else {
                available
            };
            return EnsembleResult::empty(
                regime,
                &reserve_symbol,
                format!(
                    "regime '{}' is not configured. Available regimes: {}.",
                    regime.as_str(),
                    available
                ),
            );
        }
    };

    let rationale = regime_cfg.rationale.clone();
    let blend_weights = regime_cfg.normalized();
    if blend_weights.is_empty() {
        let mut result = EnsembleResult::empty(
            regime,
            &reserve_symbol,
            format!(
                "no valid per-skill blend weights for regime '{}'.",
                regime.as_str()
            ),
        );
        result.rationale = rationale;
        return result;
    }

    let max_risk = Decimal::from_f64(config.max_risk_allocation_pct)
        .unwrap_or_else(|| Decimal::from(100));

    // Index the supplied per-skill books for lookup.
    let supplied: BTreeMap<&str, &[TargetPosition]> = per_skill_targets
        .iter()
        .map(|st| (st.skill.as_str(), st.targets.as_slice()))
        .collect();

    let mut blended_risk: BTreeMap<String, Decimal> = BTreeMap::new();
    let mut attribution: Vec<SkillContribution> = Vec::new();
    let mut loaded_any = false;

    // Iterate skills in deterministic (sorted) order — BTreeMap guarantees it.
    for (skill, &blend_weight) in &blend_weights {
        let weight = Decimal::from_f64(blend_weight).unwrap_or(Decimal::ZERO);
        let book = supplied.get(skill.as_str());

        let (risk_weight_pct, loaded) = match book {
            Some(positions) if !positions.is_empty() => {
                loaded_any = true;
                let mut risk_total = Decimal::ZERO;
                for pos in positions.iter() {
                    if pos.symbol == reserve_symbol {
                        continue;
                    }
                    risk_total += pos.weight_pct;
                    let entry = blended_risk
                        .entry(pos.symbol.clone())
                        .or_insert(Decimal::ZERO);
                    *entry += weight * pos.weight_pct;
                }
                (risk_total, true)
            }
            _ => (Decimal::ZERO, false),
        };

        attribution.push(SkillContribution {
            skill: skill.clone(),
            blend_weight: round_dp(weight, 6),
            risk_weight_pct: round_dp(risk_weight_pct, 6),
            contributed_pct: round_dp(weight * risk_weight_pct, 6),
            loaded,
            reason: if loaded {
                String::new()
            } else {
                format!("no target book supplied for skill '{skill}'")
            },
        });
    }

    if !loaded_any {
        let mut result = EnsembleResult::empty(
            regime,
            &reserve_symbol,
            format!(
                "no skill target books could be blended for regime '{}'.",
                regime.as_str()
            ),
        );
        result.rationale = rationale;
        result.attribution = attribution;
        return result;
    }

    let (blended, reserve_pct) = finalize_book(&blended_risk, &reserve_symbol, max_risk);

    let skipped: Vec<&str> = attribution
        .iter()
        .filter(|c| !c.loaded)
        .map(|c| c.skill.as_str())
        .collect();
    let reason = if skipped.is_empty() {
        String::new()
    } else {
        format!("blended with partial inputs; skipped: {}", skipped.join(", "))
    };

    EnsembleResult {
        regime,
        ok: true,
        reason,
        rationale,
        blended,
        reserve_symbol,
        reserve_pct,
        attribution,
    }
}

/// Renormalize the blended risk book to `<= max_risk` and append the reserve.
///
/// When the summed risk weight exceeds `max_risk` it is scaled down
/// proportionally so the book never over-allocates; otherwise it is kept as-is
/// and the remainder becomes the reserve. Returns the ordered position list
/// (risk positions by descending weight then symbol, reserve last) and the
/// reserve percentage. Mirrors Python `_finalize_book`.
fn finalize_book(
    blended_risk: &BTreeMap<String, Decimal>,
    reserve_symbol: &str,
    max_risk: Decimal,
) -> (Vec<TargetPosition>, Decimal) {
    let total_risk: Decimal = blended_risk.values().copied().sum();

    let (scaled, total_risk): (BTreeMap<String, Decimal>, Decimal) =
        if total_risk > max_risk && total_risk > Decimal::ZERO {
            let scale = max_risk / total_risk;
            let scaled = blended_risk
                .iter()
                .map(|(s, &w)| (s.clone(), w * scale))
                .collect();
            (scaled, max_risk)
        } else {
            (blended_risk.clone(), total_risk)
        };

    let remainder = max_risk - total_risk;
    let reserve_pct = round_dp(remainder.max(Decimal::ZERO), 4);

    let mut positions: Vec<TargetPosition> = scaled
        .into_iter()
        .map(|(symbol, weight)| TargetPosition {
            symbol,
            weight_pct: round_dp(weight, 4),
        })
        .filter(|p| p.weight_pct > Decimal::ZERO)
        .collect();

    // Descending weight, then symbol ascending for stable ordering.
    positions.sort_by(|a, b| {
        b.weight_pct
            .cmp(&a.weight_pct)
            .then_with(|| a.symbol.cmp(&b.symbol))
    });

    if reserve_pct > Decimal::ZERO {
        positions.push(TargetPosition {
            symbol: reserve_symbol.to_string(),
            weight_pct: reserve_pct,
        });
    }

    (positions, reserve_pct)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weights::{EnsembleConfig, RegimeWeights};
    use std::collections::BTreeMap;

    fn tp(symbol: &str, weight: i64) -> TargetPosition {
        TargetPosition {
            symbol: symbol.to_string(),
            weight_pct: Decimal::from(weight),
        }
    }

    fn config_with(regime: &str, weights: &[(&str, f64)]) -> EnsembleConfig {
        let mut w = BTreeMap::new();
        for (skill, weight) in weights {
            w.insert(skill.to_string(), *weight);
        }
        let mut regimes = BTreeMap::new();
        regimes.insert(
            regime.to_string(),
            RegimeWeights {
                rationale: "test".to_string(),
                weights: w,
            },
        );
        EnsembleConfig {
            version: "test".to_string(),
            reserve_symbol: "USDT".to_string(),
            max_risk_allocation_pct: 100.0,
            regimes,
        }
    }

    #[test]
    fn normalized_blend_weights_sum_to_one() {
        let rw = RegimeWeights {
            rationale: String::new(),
            weights: BTreeMap::from([
                ("a".to_string(), 0.35),
                ("b".to_string(), 0.35),
                ("c".to_string(), 0.30),
            ]),
        };
        let norm = rw.normalized();
        let sum: f64 = norm.values().sum();
        assert!((sum - 1.0).abs() < 1e-9, "weights must renormalize to 1.0");
    }

    #[test]
    fn normalized_drops_nonpositive_and_renormalizes() {
        let rw = RegimeWeights {
            rationale: String::new(),
            weights: BTreeMap::from([
                ("a".to_string(), 1.0),
                ("b".to_string(), 1.0),
                ("neg".to_string(), -5.0),
                ("zero".to_string(), 0.0),
            ]),
        };
        let norm = rw.normalized();
        assert_eq!(norm.len(), 2);
        assert!((norm["a"] - 0.5).abs() < 1e-9);
        assert!((norm["b"] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn blend_renormalizes_and_holds_reserve_remainder() {
        // Two skills, equal blend weight. Skill A holds 60% BTC, skill B 40% BTC.
        // Blended BTC = 0.5*60 + 0.5*40 = 50. Reserve = 100 - 50 = 50.
        let cfg = config_with("chop", &[("a", 0.5), ("b", 0.5)]);
        let inputs = vec![
            SkillTargets::new("a", vec![tp("BTC", 60), tp("USDT", 40)]),
            SkillTargets::new("b", vec![tp("BTC", 40), tp("USDT", 60)]),
        ];
        let result = blend_targets(&cfg, MarketRegime::Chop, &inputs);
        assert!(result.ok);
        // Risk line + reserve line.
        assert_eq!(result.blended.len(), 2);
        let btc = result
            .blended
            .iter()
            .find(|p| p.symbol == "BTC")
            .expect("BTC present");
        assert_eq!(btc.weight_pct, Decimal::from(50));
        assert_eq!(result.reserve_pct, Decimal::from(50));
        // Total of the book is exactly 100.
        let total: Decimal = result.blended.iter().map(|p| p.weight_pct).sum();
        assert_eq!(total, Decimal::from(100));
    }

    #[test]
    fn blend_scales_down_when_over_allocated() {
        // Both skills fully invested in BTC at 100% -> blended risk = 100, but
        // add a second symbol so the risk total exceeds max and must scale.
        let cfg = config_with("breakout", &[("a", 1.0)]);
        let inputs = vec![SkillTargets::new(
            "a",
            vec![tp("BTC", 80), tp("ETH", 80)],
        )];
        let result = blend_targets(&cfg, MarketRegime::Breakout, &inputs);
        assert!(result.ok);
        let total_risk: Decimal = result
            .blended
            .iter()
            .filter(|p| p.symbol != "USDT")
            .map(|p| p.weight_pct)
            .sum();
        // 160 risk scaled to 100, no reserve.
        assert_eq!(total_risk, Decimal::from(100));
        assert_eq!(result.reserve_pct, Decimal::ZERO);
        assert!(result.blended.iter().all(|p| p.symbol != "USDT"));
    }

    #[test]
    fn empty_inputs_yield_typed_failure_without_panic() {
        let cfg = config_with("risk_on", &[("a", 1.0)]);
        let result = blend_targets(&cfg, MarketRegime::RiskOn, &[]);
        assert!(!result.ok);
        assert!(result.blended.is_empty());
        assert!(!result.reason.is_empty());
    }

    #[test]
    fn unknown_regime_is_reported() {
        // Config only has "chop"; ask for risk_off.
        let cfg = config_with("chop", &[("a", 1.0)]);
        let result = blend_targets(&cfg, MarketRegime::RiskOff, &[]);
        assert!(!result.ok);
        assert!(result.reason.contains("risk_off"));
    }

    #[test]
    fn partial_inputs_are_attributed_as_skipped() {
        let cfg = config_with("chop", &[("a", 0.5), ("b", 0.5)]);
        let inputs = vec![SkillTargets::new("a", vec![tp("BTC", 50)])];
        let result = blend_targets(&cfg, MarketRegime::Chop, &inputs);
        assert!(result.ok);
        assert!(result.reason.contains("skipped"));
        let b = result
            .attribution
            .iter()
            .find(|c| c.skill == "b")
            .expect("b attributed");
        assert!(!b.loaded);
    }
}
