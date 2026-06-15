//! Bounded exponential-backoff retry for transient CMC failures.

use crate::error::CmcError;
use std::future::Future;
use std::time::Duration;

/// Retry `op` up to `max_attempts` times with exponential backoff.
/// Rate-limit errors honour the suggested retry-after.
pub async fn with_retry<T, F, Fut>(max_attempts: u32, mut op: F) -> Result<T, CmcError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, CmcError>>,
{
    let mut attempt = 0;
    loop {
        attempt += 1;
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) if attempt >= max_attempts => return Err(e),
            Err(CmcError::RateLimited(secs)) => {
                tokio::time::sleep(Duration::from_secs(secs)).await;
            }
            Err(_) => {
                let backoff = Duration::from_millis(200u64 * 2u64.pow(attempt - 1));
                tokio::time::sleep(backoff).await;
            }
        }
    }
}
