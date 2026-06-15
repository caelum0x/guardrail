use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("runtime configuration error: {0}")]
    Config(String),
}
