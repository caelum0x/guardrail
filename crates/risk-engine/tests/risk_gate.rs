//! Integration tests for the risk gate. These assert the golden rule: bad
//! orders never receive an approval.

use common::{Decimal, OrderIntent, OrderSide, QuoteSummary};
use risk_engine::{RiskContext, RiskDecision, RiskEngine, RiskPolicy};

fn engine() -> RiskEngine {
    RiskEngine::new(RiskPolicy::default())
}

fn buy(to: &str, amount: i64) -> OrderIntent {
    OrderIntent::new(OrderSide::Buy, "USDT", to, Decimal::from(amount), "test")
}

fn good_quote() -> QuoteSummary {
    QuoteSummary {
        expected_out_usd: Decimal::from(100),
        price_impact_pct: Decimal::new(1, 2), // 0.01%
        slippage_pct: Decimal::new(1, 1),     // 0.1%
        liquidity_usd: Decimal::from(1_000_000),
    }
}

#[test]
fn rejects_non_eligible_asset() {
    let decision = engine().pre_trade(&buy("FOO", 100), &RiskContext::empty(Decimal::from(10_000)));
    match decision {
        RiskDecision::Rejected { reasons } => {
            assert!(reasons.iter().any(|r| r.contains("FOO")), "got {reasons:?}");
        }
        other => panic!("expected rejection, got {other:?}"),
    }
}

#[test]
fn rejects_position_over_cap() {
    let mut ctx = RiskContext::empty(Decimal::from(10_000));
    ctx.target_position_pct = Decimal::from(50); // policy cap is 18%
    let decision = engine().pre_trade(&buy("CAKE", 100), &ctx);
    assert!(
        !decision.is_approved(),
        "over-cap position must not be approved"
    );
}

#[test]
fn rejects_daily_drawdown_breach() {
    let mut ctx = RiskContext::empty(Decimal::from(10_000));
    ctx.daily_drawdown_pct = Decimal::from(10); // policy daily cap is 7%
    let decision = engine().pre_trade(&buy("CAKE", 100), &ctx);
    assert!(
        !decision.is_approved(),
        "trading must halt past the daily loss cap"
    );
}

#[test]
fn rejects_total_drawdown_breach() {
    let mut ctx = RiskContext::empty(Decimal::from(10_000));
    ctx.total_drawdown_pct = Decimal::from(22);
    let decision = engine().pre_trade(&buy("CAKE", 100), &ctx);
    match decision {
        RiskDecision::Rejected { reasons } => {
            assert!(
                reasons.iter().any(|r| r.contains("total drawdown")),
                "got {reasons:?}"
            );
        }
        other => panic!("expected rejection, got {other:?}"),
    }
}

#[test]
fn rejects_kill_switch_drawdown_breach() {
    let mut ctx = RiskContext::empty(Decimal::from(10_000));
    ctx.total_drawdown_pct = Decimal::from(24);
    let decision = engine().pre_trade(&buy("CAKE", 100), &ctx);
    match decision {
        RiskDecision::Rejected { reasons } => {
            assert!(
                reasons.iter().any(|r| r.contains("kill switch")),
                "got {reasons:?}"
            );
        }
        other => panic!("expected rejection, got {other:?}"),
    }
}

#[test]
fn rejects_stable_reserve_breach() {
    let mut ctx = RiskContext::empty(Decimal::from(10_000));
    ctx.stable_reserve_pct = Decimal::from(5);
    let decision = engine().pre_trade(&buy("CAKE", 100), &ctx);
    match decision {
        RiskDecision::Rejected { reasons } => {
            assert!(
                reasons.iter().any(|r| r.contains("stable reserve")),
                "got {reasons:?}"
            );
        }
        other => panic!("expected rejection, got {other:?}"),
    }
}

#[test]
fn rejects_security_flags() {
    let mut ctx = RiskContext::empty(Decimal::from(10_000));
    ctx.security_flags = vec!["honeypot".to_string()];
    let decision = engine().pre_trade(&buy("CAKE", 100), &ctx);
    match decision {
        RiskDecision::Rejected { reasons } => {
            assert!(
                reasons.iter().any(|r| r.contains("honeypot")),
                "got {reasons:?}"
            );
        }
        other => panic!("expected rejection, got {other:?}"),
    }
}

#[test]
fn rejects_excess_slippage_at_final_check() {
    let ctx = RiskContext::empty(Decimal::from(10_000));
    let mut quote = good_quote();
    quote.slippage_pct = Decimal::from(5); // policy cap is 0.8%
    let decision = engine().final_quote_check(&buy("CAKE", 100), &ctx, &quote);
    assert!(!decision.is_approved(), "excess slippage must be rejected");
}

#[test]
fn rejects_missing_quote_liquidity_at_final_check() {
    let ctx = RiskContext::empty(Decimal::from(10_000));
    let mut quote = good_quote();
    quote.liquidity_usd = Decimal::ZERO;
    let decision = engine().final_quote_check(&buy("CAKE", 100), &ctx, &quote);
    match decision {
        RiskDecision::Rejected { reasons } => {
            assert!(
                reasons.iter().any(|r| r.contains("liquidity")),
                "got {reasons:?}"
            );
        }
        other => panic!("expected rejection, got {other:?}"),
    }
}

#[test]
fn clips_large_new_position() {
    let ctx = RiskContext::empty(Decimal::from(10_000));
    let decision = engine().pre_trade(&buy("CAKE", 2_000), &ctx);
    match decision {
        RiskDecision::Clipped { new_amount_usd, .. } => {
            assert_eq!(new_amount_usd, Decimal::from(1_200))
        }
        other => panic!("expected clipping, got {other:?}"),
    }
}

#[test]
fn approve_clips_large_new_position_even_with_good_quote() {
    let ctx = RiskContext::empty(Decimal::from(10_000));
    let approved = engine()
        .approve(buy("CAKE", 2_000), &ctx, &good_quote())
        .expect("oversized but otherwise clean order should be clipped");

    assert_eq!(approved.approved_amount_usd, Decimal::from(1_200));
    assert!(
        matches!(approved.decision, RiskDecision::Clipped { .. }),
        "expected clipped audit decision"
    );
}

#[test]
fn approves_clean_order() {
    let mut ctx = RiskContext::empty(Decimal::from(10_000));
    ctx.target_position_pct = Decimal::from(5);
    let approved = engine()
        .approve(buy("CAKE", 100), &ctx, &good_quote())
        .expect("clean order should be approved");
    assert_eq!(approved.approved_amount_usd, Decimal::from(100));
}
