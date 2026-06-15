//! Trust-boundary guardrails for the LLM.
//!
//! The LLM may translate mandates, explain decisions, and summarize reports.
//! It must NEVER authorize swaps, override the risk engine, edit live policy,
//! or bypass the asset allowlist / risk limits. [`authorize`] is the single
//! choke point that enforces this distinction.

use thiserror::Error;

/// Actions the LLM might attempt. Only the advisory variants are permitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmAction {
    /// Translate a natural-language mandate into candidate policy JSON.
    Translate,
    /// Explain an already-made trade decision.
    Explain,
    /// Summarize a daily report.
    Summarize,
    /// Forbidden: directly authorize a swap/trade.
    DirectSwap,
    /// Forbidden: override or disable the risk engine.
    OverrideRisk,
    /// Forbidden: edit the live policy in place.
    EditLivePolicy,
    /// Forbidden: bypass the asset allowlist or risk limits.
    BypassAllowlist,
}

impl LlmAction {
    /// Human-readable label used in error messages.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Translate => "translate",
            Self::Explain => "explain",
            Self::Summarize => "summarize",
            Self::DirectSwap => "direct swap authorization",
            Self::OverrideRisk => "risk engine override",
            Self::EditLivePolicy => "live policy edit",
            Self::BypassAllowlist => "allowlist/limit bypass",
        }
    }

    /// Whether this action stays within the LLM's advisory role.
    #[must_use]
    pub const fn is_advisory(&self) -> bool {
        matches!(self, Self::Translate | Self::Explain | Self::Summarize)
    }
}

/// Error returned when the LLM attempts a forbidden action.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum GuardrailViolation {
    /// The requested action is outside the LLM's advisory boundary.
    #[error("guardrail violation: the LLM may not perform a {0}; it is advisory only")]
    ForbiddenAction(&'static str),
}

/// Authorize an LLM action against the trust boundary.
///
/// Returns `Ok(())` only for [`LlmAction::Translate`], [`LlmAction::Explain`],
/// and [`LlmAction::Summarize`]. Every other variant is rejected with a
/// [`GuardrailViolation`].
///
/// # Errors
///
/// Returns [`GuardrailViolation::ForbiddenAction`] if `action` is not advisory.
pub fn authorize(action: &LlmAction) -> Result<(), GuardrailViolation> {
    if action.is_advisory() {
        Ok(())
    } else {
        Err(GuardrailViolation::ForbiddenAction(action.label()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_advisory_actions() {
        assert!(authorize(&LlmAction::Translate).is_ok());
        assert!(authorize(&LlmAction::Explain).is_ok());
        assert!(authorize(&LlmAction::Summarize).is_ok());
    }

    #[test]
    fn denies_direct_swap() {
        let err = authorize(&LlmAction::DirectSwap).unwrap_err();
        assert_eq!(
            err,
            GuardrailViolation::ForbiddenAction("direct swap authorization")
        );
        assert!(err.to_string().contains("advisory only"));
    }

    #[test]
    fn denies_all_forbidden_actions() {
        for action in [
            LlmAction::DirectSwap,
            LlmAction::OverrideRisk,
            LlmAction::EditLivePolicy,
            LlmAction::BypassAllowlist,
        ] {
            assert!(authorize(&action).is_err(), "{action:?} must be denied");
            assert!(!action.is_advisory());
        }
    }
}
