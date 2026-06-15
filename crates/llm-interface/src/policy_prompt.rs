//! Prompt that asks the LLM to translate a natural-language mandate into the
//! candidate policy JSON shape.
//!
//! The LLM only proposes candidate JSON. Deterministic Rust code is solely
//! responsible for validating and applying any policy; the LLM never edits a
//! live policy directly.

use crate::prompts::PromptBuilder;

/// Describes the expected policy JSON shape so the model emits something the
/// Rust validator can parse. Kept inline and stable for deterministic prompts.
const POLICY_SCHEMA_HINT: &str = concat!(
    "Return a single JSON object with these fields:\n",
    "  - \"allowlist\": array of uppercase asset symbols (strings)\n",
    "  - \"max_position_pct\": number in [0, 100]\n",
    "  - \"max_drawdown_pct\": number in [0, 100]\n",
    "  - \"rebalance\": one of \"daily\", \"weekly\", \"monthly\"\n",
    "  - \"notes\": short free-text rationale (string)\n",
    "Emit only the JSON object, with no prose before or after it.",
);

/// Build a deterministic prompt instructing the LLM to translate `mandate`
/// into candidate policy JSON for downstream Rust validation.
///
/// The returned string is advisory: it asks for *candidate* JSON only and
/// never authorizes the model to apply or edit a live policy.
#[must_use]
pub fn build_policy_prompt(mandate: &str) -> String {
    PromptBuilder::new()
        .section(
            "Task",
            "Translate the investor mandate below into candidate policy JSON.",
        )
        .section("Mandate", mandate.trim())
        .section("Output schema", POLICY_SCHEMA_HINT)
        .section(
            "Reminder",
            "This JSON is a candidate only. Rust code validates and applies it; \
             you must not assume it takes effect.",
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_mandate_text() {
        let prompt = build_policy_prompt("Invest in large-cap tech, low risk");
        assert!(prompt.contains("Invest in large-cap tech, low risk"));
    }

    #[test]
    fn includes_schema_and_candidate_language() {
        let prompt = build_policy_prompt("anything");
        assert!(prompt.contains("\"allowlist\""));
        assert!(prompt.contains("max_drawdown_pct"));
        assert!(prompt.contains("candidate"));
    }

    #[test]
    fn trims_mandate_whitespace() {
        let prompt = build_policy_prompt("   hold cash   ");
        assert!(prompt.contains("hold cash"));
        assert!(!prompt.contains("   hold cash   "));
    }
}
