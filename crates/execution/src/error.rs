use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("risk rejected order: {0}")]
    RiskRejected(String),
    #[error("TWAK execution error: {0}")]
    Twak(String),
    #[error("portfolio reconciliation error: {0}")]
    Reconciliation(String),
}
