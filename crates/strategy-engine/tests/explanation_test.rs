//! Tests for the human-readable strategy explanation builder.

use common::{Decimal, OrderIntent, OrderSide, TargetPosition};
use market_data::FearGreedSnapshot;
use strategy_engine::alpha_score::ScoredAsset;
use strategy_engine::explanation::build;
use strategy_engine::regime::MarketRegime;

fn scored(symbol: &str, score: f64) -> ScoredAsset {
    ScoredAsset {
        symbol: symbol.to_string(),
        score,
        risk_penalty: 0.0,
    }
}

#[test]
fn build_carries_regime_string_and_top_scores() {
    let scored_assets = vec![scored("AAA", 0.8), scored("BBB", 0.6)];
    let targets = vec![TargetPosition {
        symbol: "AAA".to_string(),
        weight_pct: Decimal::from(20),
    }];
    let orders = vec![OrderIntent::new(
        OrderSide::Buy,
        "USDT",
        "AAA",
        Decimal::from(200),
        "increase AAA",
    )];

    let exp = build(
        MarketRegime::RiskOn,
        &scored_assets,
        &targets,
        &orders,
        None,
    );

    assert_eq!(exp.regime, "risk_on");
    assert_eq!(exp.top_scores.len(), 2);
    assert_eq!(exp.top_scores[0].0, "AAA");
    assert_eq!(exp.top_scores[0].1, 0.8);
    assert_eq!(exp.order_count, 1);
    assert_eq!(exp.target_summary.len(), 1);
    assert_eq!(
        exp.target_summary[0],
        ("AAA".to_string(), "20%".to_string())
    );
    assert!(exp.fear_greed.is_none());
    assert!(exp.headline.contains("Risk-on"));
}

#[test]
fn build_caps_top_scores_at_five() {
    let scored_assets: Vec<ScoredAsset> = (0..8)
        .map(|i| scored(&format!("A{i}"), 1.0 - i as f64 * 0.05))
        .collect();
    let exp = build(MarketRegime::Breakout, &scored_assets, &[], &[], None);
    assert_eq!(exp.top_scores.len(), 5);
    assert_eq!(exp.regime, "breakout");
}

#[test]
fn build_includes_fear_greed_value_when_present() {
    let fg = FearGreedSnapshot {
        value: 72,
        classification: "Greed".to_string(),
        updated_ms: 0,
    };
    let exp = build(MarketRegime::RiskOff, &[], &[], &[], Some(&fg));
    assert_eq!(exp.fear_greed, Some(72));
    assert_eq!(exp.regime, "risk_off");
    assert_eq!(exp.order_count, 0);
    assert!(exp.top_scores.is_empty());
}
