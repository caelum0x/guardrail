use common::{Decimal, OrderIntent, OrderSide};
use execution::{execute_approved, quote_then_approve, ExecutionError};
use risk_engine::{RiskContext, RiskDecision, RiskEngine, RiskPolicy};
use twak_client::MockTwakClient;

fn buy(to: &str, amount: i64) -> OrderIntent {
    OrderIntent::new(
        OrderSide::Buy,
        "USDT",
        to,
        Decimal::from(amount),
        "pipeline test",
    )
}

#[tokio::test]
async fn quote_then_approve_and_execute_clean_order() {
    let twak = MockTwakClient::new();
    let risk = RiskEngine::new(RiskPolicy::default());
    let mut ctx = RiskContext::empty(Decimal::from(10_000));
    ctx.target_position_pct = Decimal::from(5);

    let approved = quote_then_approve(&twak, &risk, buy("CAKE", 100), &ctx)
        .await
        .expect("clean order should pass the pipeline");
    assert_eq!(approved.approved_amount_usd, Decimal::from(100));
    assert!(matches!(approved.decision, RiskDecision::Approved));

    let receipt = execute_approved(&twak, &approved)
        .await
        .expect("mock TWAK execution should succeed");
    assert_eq!(receipt.status, "confirmed");
    assert!(receipt.tx_hash.starts_with("0xdead"));
}

#[tokio::test]
async fn quote_then_approve_preserves_clipped_risk_decision() {
    let twak = MockTwakClient::new();
    let risk = RiskEngine::new(RiskPolicy::default());
    let ctx = RiskContext::empty(Decimal::from(10_000));

    let approved = quote_then_approve(&twak, &risk, buy("CAKE", 2_000), &ctx)
        .await
        .expect("oversized clean order should be clipped before execution");

    assert_eq!(approved.approved_amount_usd, Decimal::from(1_200));
    assert!(matches!(approved.decision, RiskDecision::Clipped { .. }));
}

#[tokio::test]
async fn quote_then_approve_rejects_before_execution_for_bad_asset() {
    let twak = MockTwakClient::new();
    let risk = RiskEngine::new(RiskPolicy::default());
    let ctx = RiskContext::empty(Decimal::from(10_000));

    let err = quote_then_approve(&twak, &risk, buy("FOO", 100), &ctx)
        .await
        .expect_err("non-eligible asset must be rejected");

    assert!(
        matches!(err, ExecutionError::RiskRejected(_)),
        "expected risk rejection, got {err:?}"
    );
}
