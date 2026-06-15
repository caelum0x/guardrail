use thiserror::Error;

/// Crate-local result alias used by `common` helpers.
pub type Result<T> = std::result::Result<T, CommonError>;

/// Errors raised while loading config or parsing shared types.
#[derive(Debug, Error)]
pub enum CommonError {
    #[error("invalid configuration: {0}")]
    Config(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
