//! Pre-trade validation of an order intent.
//!
//! This runs *before* the risk engine and the quote: it catches malformed
//! intents (empty symbols, non-positive notional, a swap whose legs are the same
//! asset) so they never consume a quote or reach the executor. It is a cheap,
//! structural gate — the risk engine remains the authority on whether an order
//! is *allowed*.

use common::OrderIntent;

/// Default policy: always fetch a quote before swapping (self-custody safety).
pub const QUOTE_BEFORE_SWAP: bool = true;

/// A structural problem with an order intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreTradeError {
    EmptyFromSymbol,
    EmptyToSymbol,
    SameSymbol,
    NonPositiveAmount,
}

impl std::fmt::Display for PreTradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            PreTradeError::EmptyFromSymbol => "order from_symbol is empty",
            PreTradeError::EmptyToSymbol => "order to_symbol is empty",
            PreTradeError::SameSymbol => "order from_symbol and to_symbol are identical",
            PreTradeError::NonPositiveAmount => "order amount_usd is not positive",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for PreTradeError {}

/// Structurally validate an order intent before it reaches risk/quote/execution.
pub fn validate_intent(intent: &OrderIntent) -> Result<(), PreTradeError> {
    if intent.from_symbol.trim().is_empty() {
        return Err(PreTradeError::EmptyFromSymbol);
    }
    if intent.to_symbol.trim().is_empty() {
        return Err(PreTradeError::EmptyToSymbol);
    }
    if intent.from_symbol.eq_ignore_ascii_case(&intent.to_symbol) {
        return Err(PreTradeError::SameSymbol);
    }
    if intent.amount_usd <= common::Decimal::ZERO {
        return Err(PreTradeError::NonPositiveAmount);
    }
    Ok(())
}

/// Whether a quote must be obtained before executing this order, given the
/// configured policy. Kept as a function so callers can extend it (e.g. always
/// quote in live mode regardless of config).
pub fn requires_quote(quote_before_swap: bool) -> bool {
    quote_before_swap || QUOTE_BEFORE_SWAP
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{Decimal, OrderSide};

    fn intent(from: &str, to: &str, amount: i64) -> OrderIntent {
        OrderIntent::new(OrderSide::Buy, from, to, Decimal::from(amount), "test")
    }

    #[test]
    fn valid_intent_passes() {
        assert!(validate_intent(&intent("USDT", "WBNB", 100)).is_ok());
    }

    #[test]
    fn rejects_same_symbol_and_bad_amount() {
        assert_eq!(
            validate_intent(&intent("WBNB", "wbnb", 100)),
            Err(PreTradeError::SameSymbol)
        );
        assert_eq!(
            validate_intent(&intent("USDT", "WBNB", 0)),
            Err(PreTradeError::NonPositiveAmount)
        );
        assert_eq!(
            validate_intent(&intent("", "WBNB", 100)),
            Err(PreTradeError::EmptyFromSymbol)
        );
    }

    #[test]
    fn quote_is_always_required_by_default() {
        assert!(requires_quote(false));
        assert!(requires_quote(true));
    }
}
