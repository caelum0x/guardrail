mod agent_card;
mod agent_services;
mod assets;
mod audit_manifest;
mod backtest;
mod bnb_sdk;
mod briefing;
mod budget;
mod cmc_capabilities;
mod commerce;
mod compete;
mod compile;
mod correlation;
mod costs;
mod drift;
mod ensemble;
mod ensemble_live;
mod equity_ta;
mod exit_triggers;
mod experiments;
mod exposure;
mod fees;
mod funding;
mod heartbeat;
mod history;
mod indicators;
mod job_simulator;
mod journal;
mod liquidity;
mod mandates;
mod optimize;
mod orderbook;
mod playbook;
mod pnl;
mod portfolio_risk;
mod presets;
mod prizes;
mod quotes;
mod rebalance;
mod regime;
mod routes;
mod scenarios;
mod scorecard;
mod sdk_catalog;
mod proof_verify;
mod server;
mod signing_policy;
mod snapshots;
mod skill;
mod skill_detail;
mod stream;
mod skills;
mod sizer;
mod sweep;
mod ta;
mod trending;
mod version;
mod walkforward;
mod wallet_controls;
mod watchlist;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    observability::tracing_setup::init();
    version::init_uptime();
    server::serve(&bind_addr()).await
}

/// Resolve the listen address, in precedence order:
/// `GUARDRAIL_API_ADDR` (full `host:port`) > `PORT` (host fixed to 0.0.0.0) > default.
fn bind_addr() -> String {
    if let Ok(addr) = std::env::var("GUARDRAIL_API_ADDR") {
        let addr = addr.trim();
        if !addr.is_empty() {
            return addr.to_string();
        }
    }
    if let Ok(port) = std::env::var("PORT") {
        let port = port.trim();
        if !port.is_empty() {
            return format!("0.0.0.0:{port}");
        }
    }
    "0.0.0.0:8080".to_string()
}
