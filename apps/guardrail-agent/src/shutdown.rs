//! Graceful-shutdown signal handling for the Guardrail agent binary.
//!
//! [`wait_for_shutdown`] resolves when the process receives an OS shutdown
//! signal so the trading loop can be raced against it with `tokio::select!`.
//! On Unix it listens for both `SIGINT` (Ctrl-C) and `SIGTERM` (the signal a
//! supervisor/container sends to stop a process); on other platforms it falls
//! back to Ctrl-C only. The runtime loop itself lives in the read-only
//! `agent-runtime` crate, so cancellation here works by dropping the in-flight
//! `run()` future when this branch of the `select!` wins.

/// Wait for the first OS shutdown signal and return its human-readable name.
///
/// Returns `Err` only if the signal handlers cannot be installed (a genuine
/// startup failure worth surfacing); a delivered signal always resolves `Ok`.
#[cfg(unix)]
pub async fn wait_for_shutdown() -> anyhow::Result<&'static str> {
    use anyhow::Context;
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigint =
        signal(SignalKind::interrupt()).context("installing SIGINT (Ctrl-C) handler")?;
    let mut sigterm = signal(SignalKind::terminate()).context("installing SIGTERM handler")?;

    tokio::select! {
        _ = sigint.recv() => Ok("SIGINT"),
        _ = sigterm.recv() => Ok("SIGTERM"),
    }
}

/// Non-Unix fallback: wait for Ctrl-C only.
#[cfg(not(unix))]
pub async fn wait_for_shutdown() -> anyhow::Result<&'static str> {
    use anyhow::Context;

    tokio::signal::ctrl_c()
        .await
        .context("installing Ctrl-C handler")?;
    Ok("Ctrl-C")
}
