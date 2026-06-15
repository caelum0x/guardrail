//! Integration tests for the backtest engine and metrics.

use backtester::{run_backtest, BacktestConfig, BacktestMetrics};
use common::{AssetCategory, Decimal, EligibleAsset};
use market_data::Universe;
use risk_engine::RiskPolicy;
use strategy_engine::StrategyConfig;

fn asset(symbol: &str, cmc_id: u64, category: AssetCategory) -> EligibleAsset {
    EligibleAsset {
        symbol: symbol.to_string(),
        cmc_id,
        chain_id: 56,
        contract_address: "0x0000000000000000000000000000000000000000".to_string(),
        decimals: 18,
        category,
        enabled: true,
        min_liquidity_usd: Decimal::from(100_000),
        min_volume_24h_usd: Decimal::from(100_000),
    }
}

fn test_universe() -> Universe {
    Universe::new(vec![
        asset("USDT", 825, AssetCategory::Stable),
        asset("WBNB", 1839, AssetCategory::Core),
        asset("CAKE", 7186, AssetCategory::DeFi),
    ])
}

#[test]
fn backtest_runs_and_produces_metrics() {
    // Constructive (greedy) sentiment + the paper-mode entry threshold so the
    // strategy actually allocates over the window.
    let strat = StrategyConfig {
        min_score_to_enter: 0.55,
        min_score_to_hold: 0.45,
        ..StrategyConfig::default()
    };
    let run = run_backtest(
        &test_universe(),
        RiskPolicy::default(),
        strat,
        BacktestConfig {
            steps: 40,
            starting_usd: Decimal::from(10_000),
            fear_greed: 80,
        },
    );

    assert_eq!(run.equity_curve.len(), 40, "one NAV point per step");
    assert_eq!(run.starting_nav_usd, Decimal::from(10_000));
    assert!(run.final_nav_usd > Decimal::ZERO, "NAV must stay positive");
    // The strategy should have transacted at least once over 40 steps.
    assert!(run.metrics.trade_count > 0, "expected some trades");
    // Win rate is a percentage in 0..=100.
    assert!(run.metrics.win_rate_pct >= Decimal::ZERO);
    assert!(run.metrics.win_rate_pct <= Decimal::from(100));
}

#[test]
fn metrics_compute_total_return_and_drawdown() {
    // Curve: 100 -> 110 -> 105 -> 120, starting at 100.
    let curve = vec![Decimal::from(110), Decimal::from(105), Decimal::from(120)];
    let m = BacktestMetrics::from_curve(Decimal::from(100), &curve, 3);
    assert_eq!(m.total_return_pct, Decimal::from(20)); // (120-100)/100
    assert_eq!(m.trade_count, 3);
    // Peak 110 -> trough 105 = ~4.545% drawdown.
    assert!(m.max_drawdown_pct > Decimal::from(4));
    assert!(m.max_drawdown_pct < Decimal::from(5));
}

#[test]
fn volatility_is_positive_for_a_non_flat_curve() {
    // Step-over-step returns vary, so the std deviation of returns is positive.
    let curve = vec![Decimal::from(110), Decimal::from(105), Decimal::from(120)];
    let m = BacktestMetrics::from_curve(Decimal::from(100), &curve, 3);
    assert!(
        m.volatility_pct > Decimal::ZERO,
        "expected positive volatility, got {}",
        m.volatility_pct
    );
}

#[test]
fn volatility_is_zero_for_a_flat_curve() {
    // Every step return is identical (0%), so the std deviation is zero.
    let curve = vec![Decimal::from(100), Decimal::from(100), Decimal::from(100)];
    let m = BacktestMetrics::from_curve(Decimal::from(100), &curve, 0);
    assert_eq!(m.volatility_pct, Decimal::ZERO);
}

#[test]
fn calmar_is_zero_when_there_is_no_drawdown() {
    // Monotonically increasing curve => no drawdown => calmar defaults to 0.
    let curve = vec![Decimal::from(110), Decimal::from(120), Decimal::from(130)];
    let m = BacktestMetrics::from_curve(Decimal::from(100), &curve, 3);
    assert_eq!(m.max_drawdown_pct, Decimal::ZERO);
    assert_eq!(m.calmar_ratio, Decimal::ZERO);
}

#[test]
fn calmar_equals_total_return_over_max_drawdown() {
    // Curve: 100 -> 110 -> 105 -> 120.
    // total_return = (120-100)/100 = 20%.
    // max drawdown = (110-105)/110 * 100 = ~4.545%.
    // calmar = total_return / max_drawdown (both positive => positive calmar).
    let curve = vec![Decimal::from(110), Decimal::from(105), Decimal::from(120)];
    let m = BacktestMetrics::from_curve(Decimal::from(100), &curve, 3);

    assert!(m.calmar_ratio > Decimal::ZERO, "calmar should be positive");
    // Reconstruct the documented relationship from the reported components.
    let expected = (m.total_return_pct / m.max_drawdown_pct).round_dp(3);
    assert_eq!(m.calmar_ratio, expected);
    // Sanity on shape: 20 / ~4.545 is roughly 4.4.
    assert!(m.calmar_ratio > Decimal::from(4));
    assert!(m.calmar_ratio < Decimal::from(5));
}
