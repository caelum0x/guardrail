mod agent_card;
mod agent_services;
mod assets;
mod audit_manifest;
mod backtest;
mod bnb_sdk;
mod briefing;
mod budget;
mod commerce;
mod compete;
mod compile;
mod costs;
mod drift;
mod exit_triggers;
mod experiments;
mod exposure;
mod funding;
mod heartbeat;
mod history;
mod indicators;
mod job_simulator;
mod liquidity;
mod mandates;
mod optimize;
mod playbook;
mod presets;
mod prizes;
mod quotes;
mod rebalance;
mod regime;
mod routes;
mod scenarios;
mod scorecard;
mod sdk_catalog;
mod server;
mod signing_policy;
mod skill;
mod sweep;
mod trending;
mod walkforward;
mod wallet_controls;
mod watchlist;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    observability::tracing_setup::init();
    server::serve("0.0.0.0:8080").await
}
