use thiserror::Error;

#[derive(Debug, Error)]
pub enum CmcError {
    #[error("http error: {0}")]
    Http(String),

    #[error("rate limited; retry after {0}s")]
    RateLimited(u64),

    #[error("missing api key")]
    MissingApiKey,

    #[error("unexpected response shape: {0}")]
    Decode(String),

    #[error("data unavailable for {0}")]
    NotFound(String),
}

impl From<reqwest::Error> for CmcError {
    fn from(e: reqwest::Error) -> Self {
        CmcError::Http(e.to_string())
    }
}
