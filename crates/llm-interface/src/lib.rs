//! LLM boundary for policy drafting and human-readable explanations.

pub mod client;
pub mod explanation_prompt;
pub mod guardrails;
pub mod policy_prompt;
pub mod prompts;
pub mod report_prompt;

pub use client::{LlmClient, LlmClientConfig, LlmError, MockLlmClient};
pub use explanation_prompt::build_explanation_prompt;
pub use guardrails::{authorize, GuardrailViolation, LlmAction};
pub use policy_prompt::build_policy_prompt;
pub use prompts::{PromptBuilder, BOUNDARY_REMINDER, SYSTEM_PROMPT};
pub use report_prompt::{build_report_prompt, DailyReportStats};
