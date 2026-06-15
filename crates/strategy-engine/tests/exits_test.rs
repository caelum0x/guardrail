//! Tests for the hard-exit rule: a position is forced out when conviction
//! drops below the hold threshold or the symbol disappears from scoring.

use strategy_engine::alpha_score::ScoredAsset;
use strategy_engine::exits::should_exit;
use strategy_engine::strategy_config::StrategyConfig;

fn scored(symbol: &str, score: f64) -> ScoredAsset {
    ScoredAsset {
        symbol: symbol.to_string(),
        score,
        risk_penalty: 0.0,
    }
}

fn cfg() -> StrategyConfig {
    StrategyConfig {
        min_score_to_hold: 0.50,
        ..Default::default()
    }
}

#[test]
fn exits_when_score_below_hold_threshold() {
    let assets = vec![scored("AAA", 0.49)];
    assert!(should_exit(&assets, "AAA", &cfg()));
}

#[test]
fn holds_when_score_above_hold_threshold() {
    let assets = vec![scored("AAA", 0.51)];
    assert!(!should_exit(&assets, "AAA", &cfg()));
}

#[test]
fn holds_when_score_exactly_at_threshold() {
    // Boundary is `<`, so equal-to-threshold is a hold, not an exit.
    let assets = vec![scored("AAA", 0.50)];
    assert!(!should_exit(&assets, "AAA", &cfg()));
}

#[test]
fn exits_when_symbol_absent_from_scored_set() {
    let assets = vec![scored("AAA", 0.90)];
    assert!(should_exit(&assets, "ZZZ", &cfg()));
}

#[test]
fn exits_when_scored_set_is_empty() {
    let assets: Vec<ScoredAsset> = vec![];
    assert!(should_exit(&assets, "AAA", &cfg()));
}
