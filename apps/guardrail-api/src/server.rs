use axum::{routing::get, Router};

pub async fn serve(addr: &str) -> anyhow::Result<()> {
    let app = build_app(crate::routes::AppState::from_env());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

pub fn build_app(state: crate::routes::AppState) -> Router {
    Router::new()
        .route("/health", get(crate::routes::health))
        .route("/portfolio", get(crate::routes::portfolio))
        .route("/trades", get(crate::routes::trades))
        .route("/signals", get(crate::routes::signals))
        .route("/risk", get(crate::routes::risk))
        .route("/alerts", get(crate::routes::alerts))
        .route(
            "/audit-manifest",
            get(crate::audit_manifest::audit_manifest),
        )
        .route(
            "/agent-services",
            get(crate::agent_services::agent_services),
        )
        .route("/agent-card", get(crate::agent_card::agent_card))
        .route(
            "/.well-known/agent-card.json",
            get(crate::agent_card::well_known_agent_card),
        )
        .route("/bnb-sdk", get(crate::bnb_sdk::bnb_sdk))
        .route("/readiness", get(crate::routes::readiness))
        .route("/events", get(crate::routes::events))
        .route("/proof", get(crate::routes::proof))
        .route("/exposure", get(crate::exposure::exposure))
        .route("/cockpit", get(crate::routes::cockpit))
        .route("/report", get(crate::routes::report_json))
        .route("/report/markdown", get(crate::routes::report_markdown))
        .route(
            "/export/submission.md",
            get(crate::routes::submission_markdown),
        )
        .route("/policy", get(crate::routes::policy))
        .route("/universe", get(crate::routes::universe))
        .route("/config", get(crate::routes::config_inventory))
        .route("/commerce", get(crate::commerce::commerce))
        .route("/ops", get(crate::routes::ops))
        .route("/playbook", get(crate::playbook::playbook))
        .route("/prizes", get(crate::prizes::prizes))
        .route("/metrics", get(crate::routes::metrics))
        .route("/backtest", get(crate::backtest::backtest))
        .route("/briefing", get(crate::briefing::briefing))
        .route("/budget", get(crate::budget::budget))
        .route("/heartbeat", get(crate::heartbeat::heartbeat))
        .route("/walkforward", get(crate::walkforward::walkforward))
        .route(
            "/wallet-controls",
            get(crate::wallet_controls::wallet_controls),
        )
        .route("/sweep", get(crate::sweep::sweep))
        .route("/history", get(crate::history::history))
        .route("/policy/compile", get(crate::compile::compile))
        .route("/assets", get(crate::assets::assets))
        .route("/costs", get(crate::costs::costs))
        .route("/drift", get(crate::drift::drift))
        .route("/exit-triggers", get(crate::exit_triggers::exit_triggers))
        .route("/indicators", get(crate::indicators::indicators))
        .route("/job-simulator", get(crate::job_simulator::job_simulator))
        .route("/liquidity", get(crate::liquidity::liquidity))
        .route("/mandates", get(crate::mandates::mandates))
        .route("/trending", get(crate::trending::trending))
        .route("/watchlist", get(crate::watchlist::watchlist))
        .route("/quotes", get(crate::quotes::quotes))
        .route("/optimize", get(crate::optimize::optimize))
        .route("/experiments", get(crate::experiments::experiments))
        .route("/skill", get(crate::skill::skill))
        .route("/scorecard", get(crate::scorecard::scorecard))
        .route("/sdk-catalog", get(crate::sdk_catalog::sdk_catalog))
        .route(
            "/signing-policy",
            get(crate::signing_policy::signing_policy),
        )
        .route("/compete", get(crate::compete::compete))
        .route("/regime", get(crate::regime::regime))
        .route("/funding", get(crate::funding::funding))
        .route("/rebalance", get(crate::rebalance::rebalance))
        .route("/scenarios", get(crate::scenarios::scenarios))
        .with_state(state)
}
