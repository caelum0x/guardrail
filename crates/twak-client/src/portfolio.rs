use common::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwakBalance {
    pub symbol: String,
    pub amount: Decimal,
    pub value_usd: Decimal,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TwakPortfolio {
    pub balances: Vec<TwakBalance>,
}
