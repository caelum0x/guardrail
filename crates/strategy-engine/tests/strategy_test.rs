//! Integration tests for the strategy engine: regime classification, position
//! capping in the allocator, and rebalance order generation past the threshold.

use common::constants::RESERVE_SYMBOL;
use common::{Decimal, OrderSide};
use market_data::RegimeInputs;
use strategy_engine::allocator::build_targets;
use strategy_engine::alpha_score::ScoredAsset;
use strategy_engine::rebalance::compute_orders;
use strategy_engine::regime::{classify, MarketRegime};
use strategy_engine::strategy_config::StrategyConfig;
use strategy_engine::target_portfolio::CurrentAllocation;

fn inputs(fear_greed: u32, breadth: i64, median: i64) -> RegimeInputs {
    RegimeInputs {
        fear_greed,
        breadth_pct: Decimal::from(breadth),
        btc_dominance_pct: Decimal::from(50),
        median_24h_return: Decimal::from(median),
    }
}

#[test]
fn classify_breakout_on_strong_broad_well_bid_advance() {
    // breadth >= 65, median > 2, fg >= 60
    assert_eq!(classify(&inputs(70, 70, 3)), MarketRegime::Breakout);
}

#[test]
fn classify_risk_on_with_healthy_appetite() {
    // breadth >= 55, fg >= 50, but not a breakout (median not > 2)
    assert_eq!(classify(&inputs(55, 60, 1)), MarketRegime::RiskOn);
}

#[test]
fn classify_risk_off_when_fearful_or_declining() {
    // low breadth triggers risk-off
    assert_eq!(classify(&inputs(50, 30, 0)), MarketRegime::RiskOff);
    // extreme fear triggers risk-off
    assert_eq!(classify(&inputs(20, 60, 1)), MarketRegime::RiskOff);
    // broadly declining median triggers risk-off
    assert_eq!(classify(&inputs(50, 50, -3)), MarketRegime::RiskOff);
}

#[test]
fn classify_chop_when_directionless() {
    // not breakout, not risk-on, not risk-off
    assert_eq!(classify(&inputs(45, 50, 0)), MarketRegime::Chop);
}

fn scored(symbol: &str, score: f64) -> ScoredAsset {
    ScoredAsset {
        symbol: symbol.to_string(),
        score,
        risk_penalty: 0.0,
    }
}

#[test]
fn allocator_caps_each_weight_at_max_position_weight_pct() {
    let cfg = StrategyConfig {
        max_position_weight_pct: 10.0,
        target_stable_reserve_pct: 0.0,
        min_score_to_enter: 0.5,
        max_positions: 5,
        ..Default::default()
    };
    // A single dominant name would otherwise grab the whole risk budget.
    let assets = vec![scored("AAA", 0.9), scored("BBB", 0.6)];
    let targets = build_targets(&assets, MarketRegime::RiskOn, &cfg);

    let cap = Decimal::from_f64_retain(cfg.max_position_weight_pct).unwrap();
    for t in &targets {
        if t.symbol == RESERVE_SYMBOL {
            continue;
        }
        assert!(
            t.weight_pct <= cap,
            "weight {} for {} exceeds cap {}",
            t.weight_pct,
            t.symbol,
            cap
        );
    }
}

#[test]
fn allocator_holds_full_reserve_when_nothing_qualifies() {
    let cfg = StrategyConfig {
        min_score_to_enter: 0.65,
        ..Default::default()
    };
    let assets = vec![scored("AAA", 0.1)];
    let targets = build_targets(&assets, MarketRegime::RiskOn, &cfg);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].symbol, RESERVE_SYMBOL);
    assert_eq!(targets[0].weight_pct, Decimal::from(100));
}

#[test]
fn rebalance_emits_buy_order_when_target_exceeds_current_past_threshold() {
    let cfg = StrategyConfig {
        rebalance_threshold_pct: 3.0,
        ..Default::default()
    };
    let targets = vec![common::TargetPosition {
        symbol: "AAA".to_string(),
        weight_pct: Decimal::from(20),
    }];
    // Currently hold nothing of AAA; delta = 20% > 3% threshold.
    let current = CurrentAllocation::new();
    let orders = compute_orders(&targets, &current, Decimal::from(1000), &cfg);

    let buy = orders
        .iter()
        .find(|o| o.to_symbol == "AAA")
        .expect("expected a buy toward AAA");
    assert_eq!(buy.side, OrderSide::Buy);
    assert_eq!(buy.from_symbol, RESERVE_SYMBOL);
    // 20% of NAV 1000 = 200 USD.
    assert_eq!(buy.amount_usd, Decimal::from(200));
}

#[test]
fn rebalance_skips_small_deltas_below_threshold() {
    let cfg = StrategyConfig {
        rebalance_threshold_pct: 3.0,
        ..Default::default()
    };
    let targets = vec![common::TargetPosition {
        symbol: "AAA".to_string(),
        weight_pct: Decimal::from(11),
    }];
    // Already hold 10%; delta = 1% < 3% threshold => no order.
    let current = CurrentAllocation::new().with_weight("AAA", Decimal::from(10));
    let orders = compute_orders(&targets, &current, Decimal::from(1000), &cfg);
    assert!(orders.is_empty(), "expected no orders, got {orders:?}");
}

#[test]
fn rebalance_exits_positions_dropped_from_target_set() {
    let cfg = StrategyConfig {
        rebalance_threshold_pct: 3.0,
        ..Default::default()
    };
    // Target set no longer includes BBB, but we still hold it.
    let targets: Vec<common::TargetPosition> = vec![];
    let current = CurrentAllocation::new().with_weight("BBB", Decimal::from(15));
    let orders = compute_orders(&targets, &current, Decimal::from(1000), &cfg);

    let exit = orders
        .iter()
        .find(|o| o.from_symbol == "BBB")
        .expect("expected an exit sell for BBB");
    assert_eq!(exit.side, OrderSide::Sell);
    assert_eq!(exit.to_symbol, RESERVE_SYMBOL);
}
