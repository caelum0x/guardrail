//! Library error types for the BNB agent crate.

/// Errors produced while building or serializing agent artifacts.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    /// JSON serialization or deserialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
