//! Integration tests for the deterministic TWAK mock executor.

use common::{Decimal, OrderIntent, OrderSide};
use twak_client::{MockTwakClient, TwakExecutor};

fn buy_intent(amount_usd: i64) -> OrderIntent {
    OrderIntent::new(
        OrderSide::Buy,
        "USDT",
        "CAKE",
        Decimal::from(amount_usd),
        "test",
    )
}

#[tokio::test]
async fn quote_swap_slippage_grows_with_amount() {
    let client = MockTwakClient::new();

    let small = client.quote_swap(&buy_intent(1_000)).await.unwrap();
    let large = client.quote_swap(&buy_intent(500_000)).await.unwrap();

    assert!(
        large.summary.slippage_pct > small.summary.slippage_pct,
        "larger trade should slip more: large={} small={}",
        large.summary.slippage_pct,
        small.summary.slippage_pct
    );
    // Price impact should also rise with size.
    assert!(large.summary.price_impact_pct > small.summary.price_impact_pct);
}

#[tokio::test]
async fn quote_swap_expected_out_is_consistent_with_slippage() {
    let client = MockTwakClient::new();
    let quote = client.quote_swap(&buy_intent(10_000)).await.unwrap();

    // expected_out = amount * (1 - slippage/100); must be below the gross.
    assert!(quote.summary.expected_out_usd < Decimal::from(10_000));
    assert!(quote.summary.expected_out_usd > Decimal::ZERO);
    assert_eq!(quote.expected_out_symbol, "CAKE");
    assert_eq!(quote.summary.expected_out_usd, quote.expected_out_amount);
}

#[tokio::test]
async fn quote_swap_larger_amount_yields_larger_gross_but_lower_efficiency() {
    let client = MockTwakClient::new();
    // Slippage is bounded below by the fixed 0.05% venue spread.
    let tiny = client.quote_swap(&buy_intent(1)).await.unwrap();
    assert!(tiny.summary.slippage_pct >= Decimal::new(5, 2));
}
