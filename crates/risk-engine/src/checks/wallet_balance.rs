use common::Decimal;

pub fn has_sufficient_balance(balance_usd: Decimal, required_usd: Decimal) -> bool {
    balance_usd >= required_usd
}
