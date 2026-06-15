//! Integration tests for portfolio accounting: NAV, reserve/weight percentages,
//! drawdown tracking, and fill application.

use common::constants::RESERVE_SYMBOL;
use common::Decimal;
use portfolio::drawdown::DrawdownTracker;
use portfolio::holding::Holding;
use portfolio::portfolio_state::PortfolioState;
use portfolio::trade_accounting::{apply_fill, Fill};

/// Build a two-asset portfolio: 100 USDT reserve + 10 units of BTC at $50.
fn sample_portfolio() -> PortfolioState {
    PortfolioState {
        holdings: vec![
            Holding::new(RESERVE_SYMBOL, Decimal::from(100), Decimal::ONE),
            Holding::new("BTC", Decimal::from(10), Decimal::from(50)),
        ],
        realized_pnl_usd: Decimal::ZERO,
    }
}

#[test]
fn nav_sums_market_value_of_all_holdings() {
    let state = sample_portfolio();
    // 100 * 1 + 10 * 50 = 600
    assert_eq!(state.nav_usd(), Decimal::from(600));
}

#[test]
fn nav_of_empty_portfolio_is_zero() {
    let state = PortfolioState::new();
    assert_eq!(state.nav_usd(), Decimal::ZERO);
}

#[test]
fn stable_reserve_pct_reflects_reserve_share_of_nav() {
    let state = sample_portfolio();
    // reserve 100 / nav 600 * 100 = 16.666...
    let pct = state.stable_reserve_pct();
    assert!(
        pct > Decimal::from(16) && pct < Decimal::from(17),
        "got {pct}"
    );
}

#[test]
fn stable_reserve_pct_is_zero_when_nav_zero() {
    let state = PortfolioState::new();
    assert_eq!(state.stable_reserve_pct(), Decimal::ZERO);
}

#[test]
fn weight_pct_reflects_symbol_share_of_nav() {
    let state = sample_portfolio();
    // BTC value 500 / nav 600 * 100 = 83.33...
    let w = state.weight_pct("BTC");
    assert!(w > Decimal::from(83) && w < Decimal::from(84), "got {w}");
}

#[test]
fn weight_pct_is_zero_for_unknown_symbol() {
    let state = sample_portfolio();
    assert_eq!(state.weight_pct("DOGE"), Decimal::ZERO);
}

#[test]
fn drawdown_total_and_daily_track_from_peak_and_day_open() {
    let now = 0i64;
    let mut tracker = DrawdownTracker::new(Decimal::from(1000), now);

    // Climb to a new peak; same UTC day so day_open stays at the start value.
    tracker.observe(Decimal::from(1200), now + 1_000);
    assert_eq!(tracker.peak_nav_usd, Decimal::from(1200));

    // Now fall back to 900.
    // total drawdown from 1200 peak: (1200-900)/1200*100 = 25%
    let total = tracker.total_drawdown_pct(Decimal::from(900));
    assert_eq!(total, Decimal::from(25));

    // daily drawdown from day-open 1000: (1000-900)/1000*100 = 10%
    let daily = tracker.daily_drawdown_pct(Decimal::from(900));
    assert_eq!(daily, Decimal::from(10));
}

#[test]
fn drawdown_is_zero_when_above_peak_and_day_open() {
    let tracker = DrawdownTracker::new(Decimal::from(1000), 0);
    assert_eq!(
        tracker.total_drawdown_pct(Decimal::from(1500)),
        Decimal::ZERO
    );
    assert_eq!(
        tracker.daily_drawdown_pct(Decimal::from(1500)),
        Decimal::ZERO
    );
}

#[test]
fn drawdown_rolls_day_boundary_and_resets_day_open() {
    // One UTC day is 86_400_000 ms. Start at epoch 0 (1970-01-01).
    let mut tracker = DrawdownTracker::new(Decimal::from(1000), 0);
    let day_one = tracker.current_day.clone();

    // Advance two days; day_open should reset to the observed NAV.
    tracker.observe(Decimal::from(800), 2 * 86_400_000);
    assert_ne!(tracker.current_day, day_one);
    assert_eq!(tracker.day_open_nav_usd, Decimal::from(800));
    // Daily drawdown measured against the new day-open is zero at that value.
    assert_eq!(
        tracker.daily_drawdown_pct(Decimal::from(800)),
        Decimal::ZERO
    );
}

#[test]
fn apply_fill_moves_balances_from_reserve_into_new_position() {
    // Start all-stable: 1000 USDT.
    let mut state = PortfolioState::seed_stable(Decimal::from(1000));

    // Buy BTC: spend 500 USDT (reserve, price 1) for BTC at $50, no fee.
    let fill = Fill {
        from_symbol: RESERVE_SYMBOL.to_string(),
        to_symbol: "BTC".to_string(),
        notional_usd: Decimal::from(500),
        to_price_usd: Decimal::from(50),
        from_price_usd: Decimal::ONE,
        fee_usd: Decimal::ZERO,
    };
    apply_fill(&mut state, &fill);

    // Reserve dropped from 1000 to 500.
    assert_eq!(
        state.get(RESERVE_SYMBOL).map(|h| h.quantity),
        Some(Decimal::from(500))
    );
    // BTC position created: 500 / 50 = 10 units.
    assert_eq!(
        state.get("BTC").map(|h| h.quantity),
        Some(Decimal::from(10))
    );
    // No realized PnL on a pure entry against stables (no price gap, no fee).
    assert_eq!(state.realized_pnl_usd, Decimal::ZERO);
}

#[test]
fn apply_fill_charges_fee_against_realized_pnl() {
    let mut state = PortfolioState::seed_stable(Decimal::from(1000));
    let fill = Fill {
        from_symbol: RESERVE_SYMBOL.to_string(),
        to_symbol: "ETH".to_string(),
        notional_usd: Decimal::from(200),
        to_price_usd: Decimal::from(10),
        from_price_usd: Decimal::ONE,
        fee_usd: Decimal::from(3),
    };
    apply_fill(&mut state, &fill);

    // Fee is a realized cost.
    assert_eq!(state.realized_pnl_usd, Decimal::from(-3));
    // Net 197 received at price 10 => 19.7 units of ETH.
    assert_eq!(
        state.get("ETH").map(|h| h.quantity),
        Some(Decimal::new(197, 1))
    );
}

#[test]
fn apply_fill_realizes_pnl_when_selling_appreciated_position() {
    // Hold 10 BTC bought at $50, now marked at $60.
    let mut state = PortfolioState {
        holdings: vec![
            Holding::new(RESERVE_SYMBOL, Decimal::from(100), Decimal::ONE),
            {
                let mut h = Holding::new("BTC", Decimal::from(10), Decimal::from(60));
                h.avg_cost_usd = Decimal::from(50);
                h
            },
        ],
        realized_pnl_usd: Decimal::ZERO,
    };

    // Sell $300 of BTC at the current $60 mark: 5 units sold.
    let fill = Fill {
        from_symbol: "BTC".to_string(),
        to_symbol: RESERVE_SYMBOL.to_string(),
        notional_usd: Decimal::from(300),
        to_price_usd: Decimal::ONE,
        from_price_usd: Decimal::from(60),
        fee_usd: Decimal::ZERO,
    };
    apply_fill(&mut state, &fill);

    // Realized PnL on 5 units * (60 - 50) = 50.
    assert_eq!(state.realized_pnl_usd, Decimal::from(50));
    // BTC reduced from 10 to 5 units.
    assert_eq!(state.get("BTC").map(|h| h.quantity), Some(Decimal::from(5)));
    // Reserve grew by the 300 notional received.
    assert_eq!(
        state.get(RESERVE_SYMBOL).map(|h| h.quantity),
        Some(Decimal::from(400))
    );
}
