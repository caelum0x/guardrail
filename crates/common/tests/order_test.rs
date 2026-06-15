//! Tests for order intents and related strategy/quote types.

use common::order::{OrderIntent, OrderSide, QuoteSummary, TargetPosition};
use common::Decimal;

#[test]
fn order_intent_new_sets_id_and_fields() {
    let intent = OrderIntent::new(
        OrderSide::Buy,
        "USDT",
        "CAKE",
        Decimal::from(250),
        "score above entry threshold",
    );

    assert!(!intent.id.is_empty());
    assert_eq!(intent.side, OrderSide::Buy);
    assert_eq!(intent.from_symbol, "USDT");
    assert_eq!(intent.to_symbol, "CAKE");
    assert_eq!(intent.amount_usd, Decimal::from(250));
    assert_eq!(intent.reason, "score above entry threshold");
}

#[test]
fn order_intent_new_generates_unique_ids() {
    let a = OrderIntent::new(OrderSide::Sell, "CAKE", "USDT", Decimal::from(1), "x");
    let b = OrderIntent::new(OrderSide::Sell, "CAKE", "USDT", Decimal::from(1), "x");
    assert_ne!(a.id, b.id);
}

#[test]
fn order_side_serializes_lowercase() {
    assert_eq!(serde_json::to_string(&OrderSide::Buy).unwrap(), "\"buy\"");
    assert_eq!(serde_json::to_string(&OrderSide::Sell).unwrap(), "\"sell\"");

    let buy: OrderSide = serde_json::from_str("\"buy\"").unwrap();
    let sell: OrderSide = serde_json::from_str("\"sell\"").unwrap();
    assert_eq!(buy, OrderSide::Buy);
    assert_eq!(sell, OrderSide::Sell);
}

#[test]
fn quote_summary_serde_round_trip() {
    let q = QuoteSummary {
        expected_out_usd: Decimal::new(99950, 2), // 999.50
        price_impact_pct: Decimal::new(15, 2),    // 0.15
        slippage_pct: Decimal::new(10, 2),        // 0.10
        liquidity_usd: Decimal::from(1_000_000),
    };
    let json = serde_json::to_string(&q).unwrap();
    let back: QuoteSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(back, q);
}

#[test]
fn target_position_holds_symbol_and_weight() {
    let tp = TargetPosition {
        symbol: "CAKE".to_string(),
        weight_pct: Decimal::from(20),
    };
    assert_eq!(tp.symbol, "CAKE");
    assert_eq!(tp.weight_pct, Decimal::from(20));

    let json = serde_json::to_string(&tp).unwrap();
    let back: TargetPosition = serde_json::from_str(&json).unwrap();
    assert_eq!(back, tp);
}
