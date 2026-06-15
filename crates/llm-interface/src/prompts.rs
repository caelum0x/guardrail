//! Shared system prompts and a small deterministic prompt-builder helper.
//!
//! Every prompt sent to the LLM is assembled from these constants so the
//! wording of the trust boundary is identical across policy translation,
//! decision explanation, and report summarization.

/// Base system prompt establishing the LLM's read-only, advisory role.
///
/// The LLM may translate, explain, and summarize. It must never authorize a
/// swap, override the risk engine, edit live policy, or bypass the allowlist
/// or limits. The guardrail layer enforces this; the prompt merely states it.
pub const SYSTEM_PROMPT: &str = concat!(
    "You are an advisory assistant for a trading system. ",
    "You may only translate mandates, explain decisions, and summarize reports. ",
    "You must NEVER authorize swaps, override risk controls, edit live policy, ",
    "or bypass the asset allowlist or risk limits. ",
    "All execution decisions are made by deterministic Rust code, not by you.",
);

/// Instruction shared by every prompt: stay within the advisory boundary.
pub const BOUNDARY_REMINDER: &str =
    "Stay strictly within your advisory role. Do not instruct any system to execute trades.";

/// Deterministic builder that assembles a prompt from a system preamble and an
/// ordered list of labelled sections.
///
/// The builder never performs I/O and produces identical output for identical
/// input, which keeps prompts reproducible and easy to assert in tests.
#[derive(Debug, Clone)]
pub struct PromptBuilder {
    system: String,
    sections: Vec<(String, String)>,
}

impl PromptBuilder {
    /// Create a builder seeded with the shared [`SYSTEM_PROMPT`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            system: SYSTEM_PROMPT.to_string(),
            sections: Vec::new(),
        }
    }

    /// Create a builder with a custom system preamble.
    #[must_use]
    pub fn with_system(system: impl Into<String>) -> Self {
        Self {
            system: system.into(),
            sections: Vec::new(),
        }
    }

    /// Append a labelled section, returning a new builder (immutable style).
    #[must_use]
    pub fn section(&self, label: impl Into<String>, body: impl Into<String>) -> Self {
        let mut sections = self.sections.clone();
        sections.push((label.into(), body.into()));
        Self {
            system: self.system.clone(),
            sections,
        }
    }

    /// Render the full prompt as a single deterministic string.
    #[must_use]
    pub fn build(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.system);
        out.push_str("\n\n");
        for (label, body) in &self.sections {
            out.push_str(label);
            out.push_str(":\n");
            out.push_str(body);
            out.push_str("\n\n");
        }
        out.push_str(BOUNDARY_REMINDER);
        out
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_states_boundary() {
        assert!(SYSTEM_PROMPT.contains("NEVER authorize swaps"));
        assert!(SYSTEM_PROMPT.contains("bypass the asset allowlist"));
    }

    #[test]
    fn builder_includes_system_and_sections() {
        let prompt = PromptBuilder::new()
            .section("Mandate", "buy tech")
            .section("Context", "low vol")
            .build();
        assert!(prompt.contains(SYSTEM_PROMPT));
        assert!(prompt.contains("Mandate:"));
        assert!(prompt.contains("buy tech"));
        assert!(prompt.contains("Context:"));
        assert!(prompt.contains(BOUNDARY_REMINDER));
    }

    #[test]
    fn builder_is_deterministic() {
        let a = PromptBuilder::new().section("X", "y").build();
        let b = PromptBuilder::new().section("X", "y").build();
        assert_eq!(a, b);
    }
}
