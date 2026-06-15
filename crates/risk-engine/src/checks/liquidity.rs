use common::{Decimal, QuoteSummary};

pub fn check_quote(quote: &QuoteSummary) -> Vec<String> {
    if quote.liquidity_usd <= Decimal::ZERO {
        vec!["quote liquidity is unavailable".into()]
    } else {
        Vec::new()
    }
}
