//! Prompt that asks the LLM to explain an already-made trade decision in
//! plain language.
//!
//! The decision has already been produced by deterministic code. The LLM only
//! narrates it; it never proposes, alters, or authorizes execution.

use crate::prompts::PromptBuilder;

/// Build a deterministic prompt asking the LLM to explain a trade decision.
///
/// * `regime` — the detected market regime label (e.g. "risk-off").
/// * `top_symbols` — the symbols the decision favored, in priority order.
/// * `order_summary` — a one-line description of the resulting order(s).
///
/// The output is purely explanatory; the prompt makes clear the decision is
/// final and made by the system, not the model.
#[must_use]
pub fn build_explanation_prompt(regime: &str, top_symbols: &[&str], order_summary: &str) -> String {
    let symbols = if top_symbols.is_empty() {
        "(none)".to_string()
    } else {
        top_symbols.join(", ")
    };

    PromptBuilder::new()
        .section(
            "Task",
            "Explain, in plain language, why the system made the trade decision below.",
        )
        .section("Detected regime", regime.trim())
        .section("Top symbols", &symbols)
        .section("Order summary", order_summary.trim())
        .section(
            "Reminder",
            "The decision is final and was made by deterministic code. \
             Describe it only; do not suggest changes or new trades.",
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_regime_and_symbols() {
        let prompt = build_explanation_prompt("risk-off", &["AAPL", "MSFT"], "Sold 10% equities");
        assert!(prompt.contains("risk-off"));
        assert!(prompt.contains("AAPL, MSFT"));
        assert!(prompt.contains("Sold 10% equities"));
    }

    #[test]
    fn handles_empty_symbols() {
        let prompt = build_explanation_prompt("neutral", &[], "No change");
        assert!(prompt.contains("(none)"));
        assert!(prompt.contains("No change"));
    }
}
