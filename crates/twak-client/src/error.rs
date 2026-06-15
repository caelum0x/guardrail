use thiserror::Error;

#[derive(Debug, Error)]
pub enum TwakError {
    #[error("TWAK transport error: {0}")]
    Transport(String),
    #[error("TWAK rejected request: {0}")]
    Rejected(String),
    #[error("TWAK response parse error: {0}")]
    Parse(String),
}
