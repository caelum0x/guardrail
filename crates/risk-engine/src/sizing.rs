use common::Decimal;

pub fn pct_of_nav(nav_usd: Decimal, pct: Decimal) -> Decimal {
    nav_usd * pct / Decimal::from(100)
}
