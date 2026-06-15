mod args;
mod bootstrap;
mod wiring;

use args::Args;
use clap::Parser;
use wiring::WiringSummary;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
    runtime.run().await
}
