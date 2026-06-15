use common::Decimal;

/// Apply a slippage haircut to a notional amount.
pub fn apply_slippage(amount: Decimal, slippage_pct: Decimal) -> Decimal {
    amount * (Decimal::from(100) - slippage_pct) / Decimal::from(100)
}

/// Estimate slippage (percent) for a trade of `amount` against a pool of
/// `liquidity_usd`: price impact grows with the fraction of the pool consumed,
/// plus a fixed venue spread.
pub fn estimate_pct(amount: Decimal, liquidity_usd: Decimal) -> Decimal {
    if liquidity_usd <= Decimal::ZERO {
        return Decimal::from(100);
    }
    let impact = amount / liquidity_usd * Decimal::from(100);
    (impact / Decimal::from(2) + Decimal::new(5, 2)).round_dp(4)
}
