//! LLM client abstraction and a deterministic, network-free mock.
//!
//! Production code depends on the [`LlmClient`] trait, never a concrete
//! network client, so the rest of the system can be tested deterministically.

use async_trait::async_trait;
use thiserror::Error;

/// Errors that an [`LlmClient`] may return.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LlmError {
    /// The prompt was empty or otherwise unusable.
    #[error("invalid prompt: {0}")]
    InvalidPrompt(String),
    /// The backend failed to produce a completion.
    #[error("completion failed: {0}")]
    Completion(String),
}

/// Optional configuration for an LLM client (model name, etc.).
#[derive(Debug, Clone)]
pub struct LlmClientConfig {
    /// Identifier of the model the client targets.
    pub model: String,
}

impl Default for LlmClientConfig {
    fn default() -> Self {
        Self {
            model: "mock".to_string(),
        }
    }
}

/// Abstraction over an LLM text-completion backend.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Complete `prompt`, returning the model's text response.
    ///
    /// # Errors
    ///
    /// Returns [`LlmError`] if the prompt is invalid or completion fails.
    async fn complete(&self, prompt: &str) -> Result<String, LlmError>;
}

/// Deterministic, network-free [`LlmClient`] used for tests and offline runs.
///
/// It returns canned text derived from a fixed reply plus a stable digest of
/// the prompt length, so output is reproducible without any I/O.
#[derive(Debug, Clone, Default)]
pub struct MockLlmClient {
    reply: String,
}

impl MockLlmClient {
    /// Create a mock that echoes a canned advisory reply.
    #[must_use]
    pub fn new() -> Self {
        Self {
            reply: "MOCK_RESPONSE: advisory text only; no execution performed.".to_string(),
        }
    }

    /// Create a mock that returns a fixed `reply` for every prompt.
    #[must_use]
    pub fn with_reply(reply: impl Into<String>) -> Self {
        Self {
            reply: reply.into(),
        }
    }
}

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(&self, prompt: &str) -> Result<String, LlmError> {
        if prompt.trim().is_empty() {
            return Err(LlmError::InvalidPrompt("prompt was empty".to_string()));
        }
        Ok(format!("{} (prompt_len={})", self.reply, prompt.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_returns_deterministic_text() {
        let client = MockLlmClient::new();
        let a = client.complete("hello").await.unwrap();
        let b = client.complete("hello").await.unwrap();
        assert_eq!(a, b);
        assert!(a.contains("MOCK_RESPONSE"));
        assert!(a.contains("prompt_len=5"));
    }

    #[tokio::test]
    async fn mock_uses_custom_reply() {
        let client = MockLlmClient::with_reply("CANNED");
        let out = client.complete("anything").await.unwrap();
        assert!(out.starts_with("CANNED"));
    }

    #[tokio::test]
    async fn mock_rejects_empty_prompt() {
        let client = MockLlmClient::new();
        let err = client.complete("   ").await.unwrap_err();
        assert!(matches!(err, LlmError::InvalidPrompt(_)));
    }
}
