//! Tests for the natural-language mandate compiler.

use policy_compiler::{compile_mandate, parse_mandate};
use rust_decimal::Decimal;

#[test]
fn parses_numeric_limits_from_mandate() {
    let mandate = "Trade CAKE and WBNB on BSC. Keep max drawdown at 20%, daily loss 6%, \
                   max position 15%, stable reserve 12%, slippage 0.5%, kill switch at 25%. \
                   Make at least 2 trades per day.";
    let policy = parse_mandate(mandate);

    assert_eq!(policy.max_total_drawdown_pct, Decimal::from(20));
    assert_eq!(policy.max_daily_drawdown_pct, Decimal::from(6));
    assert_eq!(policy.max_position_pct, Decimal::from(15));
    assert_eq!(policy.min_stable_reserve_pct, Decimal::from(12));
    assert_eq!(policy.max_slippage_pct, Decimal::new(5, 1)); // 0.5
    assert_eq!(policy.kill_switch_drawdown_pct, Decimal::from(25));
    assert_eq!(policy.daily_trade_requirement.min_trades_per_day, 2);
    assert!(policy.allowed_assets.contains(&"CAKE".to_string()));
    assert!(policy.allowed_assets.contains(&"WBNB".to_string()));
}

#[test]
fn compiles_and_hashes_a_valid_mandate() {
    let compiled = compile_mandate(
        "Trade CAKE on BSC, max drawdown 20%, kill switch 25%, stable reserve 10%.",
    )
    .expect("valid mandate should compile");
    assert_eq!(compiled.hash.len(), 64, "sha-256 hex is 64 chars");
    assert_eq!(compiled.policy.execution_layer, "twak_only");
}

#[test]
fn rejects_inconsistent_mandate() {
    // Kill switch below the total drawdown cap is invalid.
    let result = compile_mandate("max drawdown 30%, kill switch at 10%, trade CAKE");
    assert!(result.is_err(), "kill switch < total cap must be rejected");
}
