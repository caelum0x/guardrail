use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// An amount in a named currency. Defaults to USD across the system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount: Decimal,
    pub currency: String,
}

impl Money {
    pub fn usd(amount: Decimal) -> Self {
        Money {
            amount,
            currency: "USD".to_string(),
        }
    }

    pub fn zero_usd() -> Self {
        Money::usd(Decimal::ZERO)
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}
