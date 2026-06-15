//! Additional behavioral tests for the portfolio crate.
//!
//! These tests exercise the real public API of each module (verified against
//! the source) and cover trade accounting, PnL aggregation, exposure,
//! NAV history, reconciliation, and drawdown tracking.

use std::collections::HashMap;

use common::constants::RESERVE_SYMBOL;
use common::Decimal;

use portfolio::drawdown::DrawdownTracker;
use portfolio::exposure::{max_position_pct, risk_exposure_pct};
use portfolio::nav::NavHistory;
use portfolio::pnl::summarize;
use portfolio::portfolio_state::PortfolioState;
use portfolio::reconciliation::reconcile;
use portfolio::trade_accounting::{apply_fill, Fill};

fn d(n: i64) -> Decimal {
    Decimal::from(n)
}

// ---------------------------------------------------------------------------
// trade_accounting::apply_fill
// ---------------------------------------------------------------------------

#[test]
fn buy_from_reserve_creates_holding_and_reduces_reserve() {
    let mut state = PortfolioState::seed_stable(d(10_000));

    // Buy BTC with 1000 USDT, BTC at 100, no fee.
    let fill = Fill {
        from_symbol: RESERVE_SYMBOL.to_string(),
        to_symbol: "BTC".to_string(),
        notional_usd: d(1_000),
        to_price_usd: d(100),
        from_price_usd: Decimal::ONE,
        fee_usd: Decimal::ZERO,
    };
    apply_fill(&mut state, &fill);

    // Reserve reduced by 1000 (USDT price 1).
    let reserve = state.get(RESERVE_SYMBOL).expect("reserve holding present");
    assert_eq!(reserve.quantity, d(9_000));

    // BTC holding created: 1000 / 100 = 10 units at avg cost 100.
    let btc = state.get("BTC").expect("BTC holding created");
    assert_eq!(btc.quantity, d(10));
    assert_eq!(btc.avg_cost_usd, d(100));
    assert_eq!(btc.price_usd, d(100));

    // No realized PnL yet, no fee.
    assert_eq!(state.realized_pnl_usd, Decimal::ZERO);
}

#[test]
fn buy_increases_existing_holding_with_vwap() {
    let mut state = PortfolioState::seed_stable(d(10_000));

    // First buy: 10 BTC at 100.
    apply_fill(
        &mut state,
        &Fill {
            from_symbol: RESERVE_SYMBOL.to_string(),
            to_symbol: "BTC".to_string(),
            notional_usd: d(1_000),
            to_price_usd: d(100),
            from_price_usd: Decimal::ONE,
            fee_usd: Decimal::ZERO,
        },
    );

    // Second buy: 10 BTC at 200 (notional 2000).
    apply_fill(
        &mut state,
        &Fill {
            from_symbol: RESERVE_SYMBOL.to_string(),
            to_symbol: "BTC".to_string(),
            notional_usd: d(2_000),
            to_price_usd: d(200),
            from_price_usd: Decimal::ONE,
            fee_usd: Decimal::ZERO,
        },
    );

    let btc = state.get("BTC").expect("BTC holding present");
    // 10 @ 100 + 10 @ 200 => 20 units, VWAP = 3000 / 20 = 150.
    assert_eq!(btc.quantity, d(20));
    assert_eq!(btc.avg_cost_usd, d(150));
    // Reserve: 10000 - 1000 - 2000 = 7000.
    assert_eq!(state.get(RESERVE_SYMBOL).unwrap().quantity, d(7_000));
}

#[test]
fn sell_appreciated_holding_realizes_positive_pnl() {
    let mut state = PortfolioState::seed_stable(d(10_000));

    // Buy 10 BTC at 100.
    apply_fill(
        &mut state,
        &Fill {
            from_symbol: RESERVE_SYMBOL.to_string(),
            to_symbol: "BTC".to_string(),
            notional_usd: d(1_000),
            to_price_usd: d(100),
            from_price_usd: Decimal::ONE,
            fee_usd: Decimal::ZERO,
        },
    );

    // BTC appreciates to 200. The book realizes PnL against the holding's
    // current mark, so mark it up before selling.
    state.mark("BTC", d(200));

    // Sell all 10 BTC back to USDT (notional 2000).
    apply_fill(
        &mut state,
        &Fill {
            from_symbol: "BTC".to_string(),
            to_symbol: RESERVE_SYMBOL.to_string(),
            notional_usd: d(2_000),
            to_price_usd: Decimal::ONE,
            from_price_usd: d(200),
            fee_usd: Decimal::ZERO,
        },
    );

    // Realized PnL = qty_sold * (price - avg_cost) = 10 * (200 - 100) = 1000.
    assert_eq!(state.realized_pnl_usd, d(1_000));

    // BTC fully closed and dropped.
    assert!(state.get("BTC").is_none());

    // Reserve: 9000 (after buy) + 2000 (sell proceeds) = 11000.
    assert_eq!(state.get(RESERVE_SYMBOL).unwrap().quantity, d(11_000));
}

#[test]
fn fees_reduce_realized_pnl() {
    let mut state = PortfolioState::seed_stable(d(10_000));

    // Buy with a fee of 5 USD.
    apply_fill(
        &mut state,
        &Fill {
            from_symbol: RESERVE_SYMBOL.to_string(),
            to_symbol: "BTC".to_string(),
            notional_usd: d(1_000),
            to_price_usd: d(100),
            from_price_usd: Decimal::ONE,
            fee_usd: d(5),
        },
    );

    // Fee is recorded as a realized cost.
    assert_eq!(state.realized_pnl_usd, d(-5));

    // Net notional received = (1000 - 5) = 995, BTC qty = 995 / 100 = 9.95.
    let btc = state.get("BTC").expect("BTC holding present");
    assert_eq!(btc.quantity, Decimal::new(995, 2));
}

// ---------------------------------------------------------------------------
// pnl::summarize
// ---------------------------------------------------------------------------

#[test]
fn summarize_is_consistent_after_marks() {
    let mut state = PortfolioState::seed_stable(d(10_000));

    // Buy 10 BTC at 100.
    apply_fill(
        &mut state,
        &Fill {
            from_symbol: RESERVE_SYMBOL.to_string(),
            to_symbol: "BTC".to_string(),
            notional_usd: d(1_000),
            to_price_usd: d(100),
            from_price_usd: Decimal::ONE,
            fee_usd: Decimal::ZERO,
        },
    );

    // Mark BTC up to 150: unrealized = 10 * (150 - 100) = 500.
    state.mark("BTC", d(150));

    let summary = summarize(&state);
    assert_eq!(summary.realized_usd, Decimal::ZERO);
    assert_eq!(summary.unrealized_usd, d(500));
    // total = realized + unrealized, and matches the field sum.
    assert_eq!(summary.total_usd, d(500));
    assert_eq!(
        summary.total_usd,
        summary.realized_usd + summary.unrealized_usd
    );
}

#[test]
fn summarize_combines_realized_and_unrealized() {
    let mut state = PortfolioState::seed_stable(d(10_000));

    // Buy 20 BTC at 100.
    apply_fill(
        &mut state,
        &Fill {
            from_symbol: RESERVE_SYMBOL.to_string(),
            to_symbol: "BTC".to_string(),
            notional_usd: d(2_000),
            to_price_usd: d(100),
            from_price_usd: Decimal::ONE,
            fee_usd: Decimal::ZERO,
        },
    );

    // Mark up to 150 so the sell realizes against the current mark.
    state.mark("BTC", d(150));

    // Sell half (10 BTC) at 150 -> realized 10 * (150 - 100) = 500.
    apply_fill(
        &mut state,
        &Fill {
            from_symbol: "BTC".to_string(),
            to_symbol: RESERVE_SYMBOL.to_string(),
            notional_usd: d(1_500),
            to_price_usd: Decimal::ONE,
            from_price_usd: d(150),
            fee_usd: Decimal::ZERO,
        },
    );

    // Remaining 10 BTC; mark to 150 -> unrealized 10 * (150 - 100) = 500.
    state.mark("BTC", d(150));

    let summary = summarize(&state);
    assert_eq!(summary.realized_usd, d(500));
    assert_eq!(summary.unrealized_usd, d(500));
    assert_eq!(summary.total_usd, d(1_000));
}

// ---------------------------------------------------------------------------
// exposure::max_position_pct and risk_exposure_pct
// ---------------------------------------------------------------------------

#[test]
fn exposure_on_multi_holding_portfolio() {
    // NAV = 100: 50 USDT reserve, 30 BTC value, 20 ETH value.
    let mut state = PortfolioState::seed_stable(d(50));

    // Buy 30 USD worth of BTC at price 1 (qty 30).
    apply_fill(
        &mut state,
        &Fill {
            from_symbol: RESERVE_SYMBOL.to_string(),
            to_symbol: "BTC".to_string(),
            notional_usd: d(30),
            to_price_usd: Decimal::ONE,
            from_price_usd: Decimal::ONE,
            fee_usd: Decimal::ZERO,
        },
    );
    // Buy 20 USD worth of ETH at price 1 (qty 20).
    apply_fill(
        &mut state,
        &Fill {
            from_symbol: RESERVE_SYMBOL.to_string(),
            to_symbol: "ETH".to_string(),
            notional_usd: d(20),
            to_price_usd: Decimal::ONE,
            from_price_usd: Decimal::ONE,
            fee_usd: Decimal::ZERO,
        },
    );

    // NAV: reserve 0? 50 - 30 - 20 = 0 reserve, BTC 30, ETH 20 => NAV 50.
    // Recompute: reserve qty = 0, dropped? reserve kept even at zero.
    // NAV = 0 + 30 + 20 = 50.
    assert_eq!(state.nav_usd(), d(50));

    // Risk weights: BTC = 30/50 = 60%, ETH = 20/50 = 40%.
    assert_eq!(max_position_pct(&state), d(60));
    // Total at-risk exposure = 60 + 40 = 100%.
    assert_eq!(risk_exposure_pct(&state), d(100));
}

// ---------------------------------------------------------------------------
// nav::NavHistory record/latest
// ---------------------------------------------------------------------------

#[test]
fn nav_history_records_and_returns_latest() {
    let mut history = NavHistory::default();
    assert!(history.latest().is_none());

    history.record(d(1_000));
    history.record(d(1_050));

    assert_eq!(history.points.len(), 2);
    let latest = history.latest().expect("latest present");
    assert_eq!(latest.nav_usd, d(1_050));
    assert!(latest.timestamp_ms > 0);
}

// ---------------------------------------------------------------------------
// reconciliation::reconcile
// ---------------------------------------------------------------------------

#[test]
fn reconcile_matched_within_tolerance() {
    let mut state = PortfolioState::seed_stable(d(1_000));
    apply_fill(
        &mut state,
        &Fill {
            from_symbol: RESERVE_SYMBOL.to_string(),
            to_symbol: "BTC".to_string(),
            notional_usd: d(100),
            to_price_usd: d(10),
            from_price_usd: Decimal::ONE,
            fee_usd: Decimal::ZERO,
        },
    );
    // Internal: USDT 900, BTC 10.

    let mut external: HashMap<String, Decimal> = HashMap::new();
    external.insert(RESERVE_SYMBOL.to_string(), Decimal::new(9005, 1)); // 900.5
    external.insert("BTC".to_string(), Decimal::new(1001, 2)); // 10.01

    // Tolerance 1.0 absolute; both diffs (0.5 and 0.01) are within tolerance.
    let report = reconcile(&state, &external, Decimal::ONE);
    assert!(report.matched);
    assert!(report.drifts.is_empty());
}

#[test]
fn reconcile_reports_drift_beyond_tolerance() {
    let mut state = PortfolioState::seed_stable(d(1_000));
    apply_fill(
        &mut state,
        &Fill {
            from_symbol: RESERVE_SYMBOL.to_string(),
            to_symbol: "BTC".to_string(),
            notional_usd: d(100),
            to_price_usd: d(10),
            from_price_usd: Decimal::ONE,
            fee_usd: Decimal::ZERO,
        },
    );
    // Internal: USDT 900, BTC 10.

    let mut external: HashMap<String, Decimal> = HashMap::new();
    external.insert(RESERVE_SYMBOL.to_string(), d(900)); // matches
    external.insert("BTC".to_string(), d(8)); // diff = 2, beyond tolerance

    let report = reconcile(&state, &external, Decimal::ONE);
    assert!(!report.matched);
    assert_eq!(report.drifts.len(), 1);

    let drift = &report.drifts[0];
    assert_eq!(drift.symbol, "BTC");
    assert_eq!(drift.internal_qty, d(10));
    assert_eq!(drift.external_qty, d(8));
    assert_eq!(drift.abs_diff, d(2));
}

#[test]
fn reconcile_detects_symbol_only_on_external_side() {
    let state = PortfolioState::seed_stable(d(1_000));

    let mut external: HashMap<String, Decimal> = HashMap::new();
    external.insert(RESERVE_SYMBOL.to_string(), d(1_000)); // matches internal
    external.insert("BTC".to_string(), d(5)); // not held internally

    let report = reconcile(&state, &external, Decimal::ZERO);
    assert!(!report.matched);
    assert_eq!(report.drifts.len(), 1);
    let drift = &report.drifts[0];
    assert_eq!(drift.symbol, "BTC");
    assert_eq!(drift.internal_qty, Decimal::ZERO);
    assert_eq!(drift.external_qty, d(5));
    assert_eq!(drift.abs_diff, d(5));
}

// ---------------------------------------------------------------------------
// drawdown::DrawdownTracker
// ---------------------------------------------------------------------------

const MS_PER_DAY: i64 = 86_400_000;

#[test]
fn drawdown_total_from_peak() {
    let mut tracker = DrawdownTracker::new(d(1_000), 0);

    // NAV climbs to a new peak.
    tracker.observe(d(1_200), 1_000);
    assert_eq!(tracker.peak_nav_usd, d(1_200));

    // NAV drops to 900: total drawdown = (1200 - 900) / 1200 * 100 = 25%.
    assert_eq!(tracker.total_drawdown_pct(d(900)), d(25));

    // Above peak => clamped to zero.
    assert_eq!(tracker.total_drawdown_pct(d(1_300)), Decimal::ZERO);
}

#[test]
fn drawdown_daily_from_day_open() {
    let tracker = DrawdownTracker::new(d(1_000), 0);

    // Same day, NAV drops to 950: daily = (1000 - 950) / 1000 * 100 = 5%.
    assert_eq!(tracker.daily_drawdown_pct(d(950)), d(5));
    // Above day open => clamped to zero.
    assert_eq!(tracker.daily_drawdown_pct(d(1_100)), Decimal::ZERO);
}

#[test]
fn drawdown_day_roll_resets_day_open() {
    let mut tracker = DrawdownTracker::new(d(1_000), 0);
    let day0 = tracker.current_day.clone();

    // Observe on the next UTC day with a lower NAV: day open resets to it.
    tracker.observe(d(800), MS_PER_DAY);
    assert_ne!(tracker.current_day, day0);
    assert_eq!(tracker.day_open_nav_usd, d(800));

    // Daily drawdown is now measured from the new day open (800), not 1000.
    assert_eq!(tracker.daily_drawdown_pct(d(800)), Decimal::ZERO);
    // Drop to 760 within the new day: (800 - 760) / 800 * 100 = 5%.
    assert_eq!(tracker.daily_drawdown_pct(d(760)), d(5));

    // Peak persists across the day roll (peak stays 1000 since 800 < 1000).
    assert_eq!(tracker.peak_nav_usd, d(1_000));
    assert_eq!(tracker.total_drawdown_pct(d(800)), d(20));
}
