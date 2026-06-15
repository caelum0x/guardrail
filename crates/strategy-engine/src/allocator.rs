//! Portfolio construction: turn ranked scores into target weights, respecting
//! the regime's exposure multiplier and the stable reserve floor.

use crate::alpha_score::ScoredAsset;
use crate::regime::MarketRegime;
use crate::strategy_config::StrategyConfig;
use common::constants::RESERVE_SYMBOL;
use common::{Decimal, TargetPosition};
use portfolio_optimizer::AllocationMethod;
use rust_decimal::prelude::FromPrimitive;

/// Build score-proportional target weights for the top assets.
pub fn build_targets(
    scored: &[ScoredAsset],
    regime: MarketRegime,
    cfg: &StrategyConfig,
) -> Vec<TargetPosition> {
    // Select entries above the entry threshold, capped at max_positions.
    let selected: Vec<&ScoredAsset> = scored
        .iter()
        .filter(|s| s.score >= cfg.min_score_to_enter)
        .take(cfg.max_positions as usize)
        .collect();

    let mut targets = Vec::new();

    if selected.is_empty() {
        // Nothing qualifies: hold full reserve.
        targets.push(TargetPosition {
            symbol: RESERVE_SYMBOL.to_string(),
            weight_pct: Decimal::from(100),
        });
        return targets;
    }

    // Risk budget after reserving stables, scaled by the regime.
    let reserve = cfg.target_stable_reserve_pct;
    let risk_budget =
        ((100.0 - reserve) * regime.exposure_multiplier()).clamp(0.0, 100.0 - reserve);

    let mut allocated = 0.0;

    if cfg.allocation_method == AllocationMethod::ScoreProportional {
        // EXISTING logic, preserved byte-for-byte to keep current behavior.
        let score_total: f64 = selected.iter().map(|s| s.score).sum();

        for s in &selected {
            let raw_weight = if score_total > 0.0 {
                risk_budget * (s.score / score_total)
            } else {
                0.0
            };
            // Never propose a position above the per-name cap; the surplus falls
            // back to the stable reserve rather than being rejected by risk.
            let weight = raw_weight.min(cfg.max_position_weight_pct);
            allocated += weight;
            targets.push(TargetPosition {
                symbol: s.symbol.clone(),
                weight_pct: Decimal::from_f64(weight)
                    .unwrap_or(Decimal::ZERO)
                    .round_dp(2),
            });
        }
    } else {
        // Alternative allocation methods: delegate per-name weighting to the
        // portfolio-optimizer over the selected assets' scores and a derived
        // risk proxy, scaled to the same risk_budget.
        let scores: Vec<f64> = selected.iter().map(|s| s.score).collect();
        // Rough risk proxy: higher score => lower assumed volatility. Clamped
        // to stay strictly positive so inverse-vol methods stay well-defined.
        let vols: Vec<f64> = selected
            .iter()
            .map(|s| (1.0 - s.score).clamp(0.01, 1.0))
            .collect();
        let weights =
            portfolio_optimizer::allocate(cfg.allocation_method, &scores, &vols, risk_budget);

        for (s, &raw_weight) in selected.iter().zip(weights.iter()) {
            // Same per-name cap as the score-proportional path; surplus falls
            // back to the stable reserve.
            let weight = raw_weight.min(cfg.max_position_weight_pct);
            allocated += weight;
            targets.push(TargetPosition {
                symbol: s.symbol.clone(),
                weight_pct: Decimal::from_f64(weight)
                    .unwrap_or(Decimal::ZERO)
                    .round_dp(2),
            });
        }
    }

    // Remainder goes to the stable reserve.
    let stable_weight = (100.0 - allocated).max(0.0);
    targets.push(TargetPosition {
        symbol: RESERVE_SYMBOL.to_string(),
        weight_pct: Decimal::from_f64(stable_weight)
            .unwrap_or(Decimal::ZERO)
            .round_dp(2),
    });

    targets
}
