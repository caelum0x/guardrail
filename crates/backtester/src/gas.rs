use common::Decimal;

/// Gas cost in USD from gas units and a USD gas price.
pub fn gas_cost_usd(gas_used: Decimal, gas_price_usd: Decimal) -> Decimal {
    gas_used * gas_price_usd
}

/// Flat per-swap gas estimate used by the backtest simulator (BSC is cheap).
pub fn fixed_gas_usd() -> Decimal {
    Decimal::new(35, 2) // $0.35
}
