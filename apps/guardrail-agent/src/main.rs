mod args;
mod bootstrap;
mod shutdown;
mod wiring;

use anyhow::Context;
use args::Args;
use clap::Parser;
use std::process::ExitCode;
use wiring::WiringSummary;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // The runtime returned an error (or startup failed). Log the full
            // error chain with context so an operator can diagnose it, then exit
            // non-zero so supervisors (systemd, k8s, CI) see the failure.
            tracing::error!(error = ?err, "guardrail-agent terminated with error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

/// Real startup + supervised run. Kept separate from `main` so the top level can
/// map any error to a non-zero [`ExitCode`] after logging it with full context.
async fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    observability::tracing_setup::init();

    let settings = common::Settings::load(&args.config)?;
    tracing::info!("{}", bootstrap::startup_banner(&settings));

    // Resolve and report exactly how the loop will be wired, then validate the
    // configuration before any money-moving component is constructed.
    let wiring = WiringSummary::resolve(&settings);
    tracing::info!("\n{wiring}");
    bootstrap::preflight(&settings, &args.config, &wiring)?;

    let runtime = agent_runtime::AgentRuntime::new(settings);

    // Run the trading loop, but race it against OS shutdown signals so a
    // Ctrl-C / SIGTERM produces a clean, logged exit instead of a hard kill.
    // The runtime's own loop (bounded paper run or live loop) lives in the
    // read-only `agent-runtime` crate; we only supervise it from the binary.
    tokio::select! {
        biased;

        result = runtime.run() => {
            // run() finished on its own (e.g. a bounded paper run completed, or
            // it errored). Propagate the result so success exits 0 and an error
            // is logged + mapped to a non-zero exit by `main`.
            result.context("agent runtime exited with an error")?;
            tracing::info!("agent runtime completed; shutting down");
        }
        signal = shutdown::wait_for_shutdown() => {
            let signal = signal.context("failed to install shutdown signal handlers")?;
            tracing::info!(signal = %signal, "shutdown signal received, stopping");
        }
    }

    Ok(())
}
