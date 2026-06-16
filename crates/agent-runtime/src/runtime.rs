//! The autonomous runtime: wires every crate into a working trading loop.
//!
//! One cycle: pull market data (CMC) -> normalize snapshot -> features +
//! strategy -> per-order risk check -> TWAK quote -> final risk approval ->
//! TWAK execute -> portfolio reconcile -> event log. The risk engine is the
//! only gate to execution; nothing trades without an approval.

use crate::trading_loop::DEFAULT_STRATEGY_LOOP_SECONDS;
use cmc_client::{CmcDataSource, CmcTransport, MockCmcClient};
use common::constants::RESERVE_SYMBOL;
use common::decimal::to_f64;
use common::ids::new_run_id;
use common::time::now_ms;
use common::{OrderIntent, OrderSide, Settings};
use event_store::{AgentEvent, EventRepository, SqliteEventRepository};
use llm_interface::{authorize, build_explanation_prompt, LlmAction, LlmClient, MockLlmClient};
use market_data::validator::{self, SnapshotValidity};
use market_data::{MarketSnapshot, SnapshotBuilder, Universe};
use portfolio::trade_accounting::{apply_fill, Fill};
use portfolio::{DrawdownTracker, PortfolioState};
use portfolio_optimizer::AllocationMethod;
use risk_engine::{RiskContext, RiskDecision, RiskEngine, RiskPolicy};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use strategy_engine::{CurrentAllocation, StrategyConfig, StrategyEngine};
use twak_client::{SwapQuote, TwakExecutor, TwakTransport};

/// Default eligible-asset universe path (overridable via env).
const DEFAULT_UNIVERSE_PATH: &str = "configs/eligible_assets.bsc.json";
/// Paper-mode starting balance, all in the stable reserve.
const PAPER_START_USD: i64 = 10_000;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub settings: Settings,
}

pub struct AgentRuntime {
    config: RuntimeConfig,
    /// Advisory LLM client for human-readable cycle commentary. Constructed
    /// once; deterministic and network-free so paper mode stays offline. The
    /// LLM is advisory only and never authorizes or alters any trade.
    llm: MockLlmClient,
}

impl AgentRuntime {
    pub fn new(settings: Settings) -> Self {
        Self {
            config: RuntimeConfig { settings },
            llm: MockLlmClient::new(),
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.config.settings
    }

    /// Run the agent. In paper/backtest mode it runs a bounded number of cycles
    /// (env `GUARDRAIL_CYCLES`, default 4); in live mode it loops on the
    /// configured strategy interval until interrupted.
    pub async fn run(&self) -> anyhow::Result<()> {
        let s = &self.config.settings;
        tracing::info!(mode = %s.app.mode, name = %s.app.name, "agent runtime starting");

        // --- Wiring -------------------------------------------------------
        let universe_path =
            std::env::var("GUARDRAIL_UNIVERSE").unwrap_or_else(|_| DEFAULT_UNIVERSE_PATH.into());
        let universe = Universe::load(&universe_path)
            .map_err(|e| anyhow::anyhow!("failed to load universe {universe_path}: {e}"))?;

        let policy_raw = std::fs::read_to_string(&s.risk.policy_path).map_err(|e| {
            anyhow::anyhow!("failed to read risk policy {}: {e}", s.risk.policy_path)
        })?;
        let policy = RiskPolicy::from_json_str(&policy_raw)?;

        let (data_source, cmc_transport) = build_data_source(s)?;
        let (executor, twak_transport) = build_executor(s)?;
        // Surface the resolved transports so operators can see at a glance which
        // data source and executor are live vs mock for this run.
        tracing::info!(
            cmc_transport,
            twak_transport,
            "transports resolved"
        );
        // Size positions just under the risk cap so targets are not rejected.
        let position_cap = (to_f64(policy.max_position_pct) - 1.0).max(1.0);
        let strategy = StrategyEngine::new(build_strategy_config(s, position_cap));
        let risk = RiskEngine::new(policy);

        // BNB agent identity + on-chain proof commitments (deterministic, no chain calls).
        let wallet = executor
            .wallet_address()
            .await
            .map(|a| a.to_string())
            .unwrap_or_default();
        let policy_hash = bnb_agent::sha256_hex_str(&policy_raw);
        let agent_id = bnb_agent::AgentIdentity::new(s.app.name.clone(), wallet.clone()).agent_id();

        let mut portfolio = PortfolioState::seed_stable(Decimal::from(PAPER_START_USD));
        let mut drawdown = DrawdownTracker::new(portfolio.nav_usd(), now_ms());
        let mut events = RuntimeEventLog::new(&s.app.database_url);

        let run_id = new_run_id();
        let mut meta = RunMeta {
            run_id: run_id.clone(),
            mode: s.app.mode.clone(),
            wallet_address: wallet.clone(),
            starting_nav_usd: Decimal::from(PAPER_START_USD),
            policy_hash: policy_hash.clone(),
            agent_id: agent_id.clone(),
            registration_tx: None,
        };
        events.append(
            &run_id,
            AgentEvent::AgentStarted,
            json!({ "mode": s.app.mode, "agent_id": agent_id, "wallet": wallet, "policy_hash": policy_hash, "cmc_transport": cmc_transport, "twak_transport": twak_transport }),
        );

        // Track 1: register the competition wallet before trading.
        if s.twak.competition_register_enabled {
            match executor.register_competition().await {
                Ok(rcpt) => {
                    tracing::info!(tx = %rcpt.tx_hash, "competition registration submitted");
                    // Only surface a registration tx as a proof artifact when it
                    // is a real on-chain (live) registration. In paper the mock
                    // executor returns a synthetic hash; we log the event for the
                    // demo but never present it as an anchored on-chain tx.
                    if s.app.is_live() {
                        meta.registration_tx = Some(rcpt.tx_hash.clone());
                    }
                    events.append(
                        &run_id,
                        AgentEvent::TxConfirmed,
                        json!({ "competition_tx": rcpt.tx_hash }),
                    );
                }
                Err(e) => tracing::warn!(error = %e, "competition registration failed"),
            }
        }

        // BNB SDK: anchor the agent's ERC-8004 identity on-chain. Money/gas
        // action, so it is doubly gated — live mode AND an explicit operator
        // opt-in (`GUARDRAIL_ANCHOR_IDENTITY=1`) — and the CLI transport itself
        // refuses to mint without autonomous mode + a wallet password. Surfaced
        // as a proof artifact via the same TxConfirmed event the API reads.
        let anchor_identity = std::env::var("GUARDRAIL_ANCHOR_IDENTITY")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if s.app.is_live() && anchor_identity {
            let uri = format!(
                "data:application/json,{{\"agent_id\":\"{}\",\"policy_hash\":\"{}\"}}",
                meta.agent_id, meta.policy_hash
            );
            let metadata = vec![
                ("agent_id".to_string(), meta.agent_id.clone()),
                ("policy_hash".to_string(), meta.policy_hash.clone()),
            ];
            match executor.anchor_identity(&uri, &metadata).await {
                Ok(id) => {
                    tracing::info!(
                        agent_id = ?id.agent_id,
                        tx = ?id.tx_hash,
                        "ERC-8004 identity anchored on-chain"
                    );
                    events.append(
                        &run_id,
                        AgentEvent::TxConfirmed,
                        json!({
                            "erc8004_agent_id": id.agent_id,
                            "erc8004_tx": id.tx_hash,
                        }),
                    );
                }
                Err(e) => tracing::warn!(error = %e, "ERC-8004 identity anchor skipped"),
            }
        }

        // --- Loop control -------------------------------------------------
        let cycles: u32 = std::env::var("GUARDRAIL_CYCLES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(if s.app.is_live() { 0 } else { 4 });
        let interval_secs = if s.app.is_live() {
            s.strategy.loop_interval_seconds
        } else {
            1
        }
        .clamp(1, DEFAULT_STRATEGY_LOOP_SECONDS);
        let interval = Duration::from_secs(interval_secs);

        let mut cycle = 0u32;
        let mut kill_engaged = false;
        let mut last_commentary = String::new();
        loop {
            cycle += 1;
            tracing::info!(cycle, "strategy cycle begin");
            if let Err(e) = self
                .run_cycle(
                    &meta,
                    &universe,
                    data_source.as_ref(),
                    executor.as_ref(),
                    &risk,
                    &strategy,
                    &mut portfolio,
                    &mut drawdown,
                    &mut events,
                    &mut kill_engaged,
                    &mut last_commentary,
                )
                .await
            {
                tracing::error!(error = %e, "strategy cycle failed");
            }

            if cycles != 0 && cycle >= cycles {
                break;
            }
            tokio::time::sleep(interval).await;
        }

        // --- Final report + BNB proof -------------------------------------
        let nav = portfolio.nav_usd();
        let core = json!({
            "run_id": run_id,
            "cycles": cycle,
            "final_nav_usd": nav.to_string(),
            "total_drawdown_pct": drawdown.total_drawdown_pct(nav).to_string(),
            "events": events.len(),
        });
        let report_hash = bnb_agent::sha256_hex_str(&core.to_string());
        let mut proof = bnb_agent::AgentProof::new(&agent_id, &wallet, &policy_hash, &report_hash);
        if let Some(tx) = &meta.registration_tx {
            proof = proof.with_registration_tx(tx.clone());
        }

        let mut summary = core;
        summary["agent_id"] = json!(agent_id);
        summary["wallet_address"] = json!(wallet);
        summary["policy_hash"] = json!(policy_hash);
        summary["report_hash"] = json!(report_hash);
        summary["address_url"] = json!(proof.address_url());
        if let Some(tx) = &meta.registration_tx {
            summary["registration_tx"] = json!(tx);
        }
        if let Some(tx_url) = proof.tx_url() {
            summary["registration_tx_url"] = json!(tx_url);
        }

        events.append(&run_id, AgentEvent::AgentReportPublished, summary.clone());
        write_run_report(
            &meta,
            &portfolio,
            &drawdown,
            "final",
            events.len(),
            kill_engaged,
            &last_commentary,
        );
        tracing::info!(report = %summary, "agent run complete");
        Ok(())
    }

    /// Execute a single trading cycle.
    #[allow(clippy::too_many_arguments)]
    async fn run_cycle(
        &self,
        meta: &RunMeta,
        universe: &Universe,
        data_source: &dyn CmcDataSource,
        executor: &dyn TwakExecutor,
        risk: &RiskEngine,
        strategy: &StrategyEngine,
        portfolio: &mut PortfolioState,
        drawdown: &mut DrawdownTracker,
        events: &mut RuntimeEventLog,
        kill: &mut bool,
        last_commentary: &mut String,
    ) -> anyhow::Result<()> {
        let run_id: &str = &meta.run_id;
        // 1. Market data -> snapshot.
        let snapshot = SnapshotBuilder::new(data_source, universe).build().await?;
        match validator::validate(&snapshot) {
            SnapshotValidity::Ok => {}
            other => {
                tracing::warn!(?other, "snapshot invalid; skipping cycle");
                return Ok(());
            }
        }
        events.append(
            run_id,
            AgentEvent::MarketSnapshotReceived,
            json!({ "assets": snapshot.assets.len(), "ts": snapshot.timestamp_ms }),
        );
        persist_snapshot(run_id, &snapshot);

        // 2. Mark the book to current prices and refresh drawdown.
        let prices = price_map(&snapshot);
        portfolio.mark_all(&prices);
        let nav = portfolio.nav_usd();
        drawdown.observe(nav, now_ms());

        // 2b. Risk monitor: halt on the kill-switch limit, throttle on the soft
        //     (total drawdown) limit. Once the kill switch engages it stays
        //     engaged for the remainder of the run.
        let total_dd = drawdown.total_drawdown_pct(nav);
        let policy = risk.policy();
        if !*kill
            && risk_engine::kill_switch::should_trigger(total_dd, policy.kill_switch_drawdown_pct)
        {
            *kill = true;
            tracing::error!(drawdown = %total_dd, "kill switch engaged — halting trading");
            events.append(
                run_id,
                AgentEvent::KillSwitchTriggered,
                json!({ "total_drawdown_pct": total_dd.to_string(), "limit_pct": policy.kill_switch_drawdown_pct.to_string() }),
            );
        }
        if *kill {
            write_run_report(meta, portfolio, drawdown, "halted", events.len(), true, "");
            return Ok(());
        }
        let throttled = total_dd >= policy.max_total_drawdown_pct;
        if throttled {
            events.append(
                run_id,
                AgentEvent::DrawdownThrottleActivated,
                json!({ "total_drawdown_pct": total_dd.to_string(), "limit_pct": policy.max_total_drawdown_pct.to_string() }),
            );
        }

        // 2c. Protective exits: force-sell any non-reserve position whose
        //     unrealized P&L has breached the stop-loss or take-profit
        //     thresholds, regardless of strategy targets. Collect the intents
        //     first so the immutable read of `portfolio.holdings` is dropped
        //     before `process_order` needs `&mut portfolio`.
        let exit_cfg = strategy.config();
        let exit_intents: Vec<OrderIntent> = portfolio
            .holdings
            .iter()
            .filter(|h| h.symbol != RESERVE_SYMBOL && h.quantity > Decimal::ZERO)
            .filter_map(|h| {
                let reason = if strategy_engine::exits::stop_loss_hit(
                    h.avg_cost_usd,
                    h.price_usd,
                    exit_cfg.stop_loss_pct,
                ) {
                    Some("stop-loss")
                } else if strategy_engine::exits::take_profit_hit(
                    h.avg_cost_usd,
                    h.price_usd,
                    exit_cfg.take_profit_pct,
                ) {
                    Some("take-profit")
                } else {
                    None
                };
                reason.map(|r| {
                    OrderIntent::new(
                        OrderSide::Sell,
                        h.symbol.clone(),
                        RESERVE_SYMBOL,
                        h.market_value_usd(),
                        r,
                    )
                })
            })
            .collect();

        let mut executed = 0u32;
        for intent in &exit_intents {
            tracing::info!(symbol = %intent.from_symbol, reason = %intent.reason, "protective exit triggered");
            if self
                .process_order(
                    run_id, intent, &snapshot, executor, risk, portfolio, drawdown, events,
                )
                .await
            {
                executed += 1;
            }
        }

        // 3. Strategy decision.
        let current = CurrentAllocation {
            weights_pct: portfolio.risk_weights_pct(),
        };
        let decision = strategy.decide(&snapshot, &current, nav);
        let ensemble = ensemble_routing(decision.regime.as_str());
        if let Some(ref routing) = ensemble {
            tracing::info!(regime = decision.regime.as_str(), routing = %routing, "ensemble routing");
        }
        events.append(
            run_id,
            AgentEvent::RegimeClassified,
            json!({ "regime": decision.regime.as_str(), "ensemble": ensemble }),
        );

        // Advisory commentary: ask the LLM to narrate the already-made decision
        // in plain language. This is an Explain action — guardrail-gated as
        // advisory only; it never authorizes, alters, or proposes any trade.
        let commentary = self
            .cycle_commentary(
                decision.regime.as_str(),
                &decision.explanation.top_scores,
                decision.proposed_orders.len(),
            )
            .await;
        *last_commentary = commentary.clone();

        events.append(
            run_id,
            AgentEvent::PortfolioTargetComputed,
            json!({
                "headline": decision.explanation.headline,
                "orders": decision.proposed_orders.len(),
                "commentary": commentary,
            }),
        );
        // Per-asset alpha scores so the /signals surface reflects real scoring.
        for (symbol, score) in &decision.explanation.top_scores {
            events.append(
                run_id,
                AgentEvent::AssetScored,
                json!({ "symbol": symbol, "score": score }),
            );
        }
        tracing::info!(
            regime = decision.regime.as_str(),
            orders = decision.proposed_orders.len(),
            nav = %nav,
            "{}",
            decision.explanation.headline
        );

        // 4. Execute each proposed order through the risk gate. While throttled
        //    the book is reduce-only: new buys are skipped, sells/trims allowed.
        for intent in &decision.proposed_orders {
            if throttled && intent.side == OrderSide::Buy {
                continue;
            }
            if self
                .process_order(
                    run_id, intent, &snapshot, executor, risk, portfolio, drawdown, events,
                )
                .await
            {
                executed += 1;
            }
        }

        // 5. Daily-trade requirement: if the strategy traded nothing this cycle,
        //    inject a small compliant heartbeat so Track 1 activity is met.
        let daily = &risk.policy().daily_trade_requirement;
        if daily.enabled && executed == 0 {
            if let Some(hb) = heartbeat_intent(portfolio, &snapshot, daily.max_heartbeat_trade_pct)
            {
                tracing::info!(to = %hb.to_symbol, from = %hb.from_symbol, "no trades this cycle; issuing daily-trade heartbeat");
                if self
                    .process_order(
                        run_id, &hb, &snapshot, executor, risk, portfolio, drawdown, events,
                    )
                    .await
                {
                    executed += 1;
                }
            }
        }
        if executed > 0 {
            events.append(
                run_id,
                AgentEvent::DailyTradeRequirementSatisfied,
                json!({ "trades": executed }),
            );
        }

        events.append(
            run_id,
            AgentEvent::PortfolioReconciled,
            json!({ "nav_usd": portfolio.nav_usd().to_string(), "positions": portfolio.position_count() }),
        );

        // Publish a fresh run report for the monitor sidecar + dashboard.
        write_run_report(
            meta,
            portfolio,
            drawdown,
            decision.regime.as_str(),
            events.len(),
            *kill,
            &commentary,
        );
        Ok(())
    }

    /// Build a plain-language commentary for the cycle by asking the advisory
    /// LLM to narrate the already-made decision.
    ///
    /// This calls [`LlmAction::Explain`], which the guardrail layer permits; the
    /// LLM is advisory only and never authorizes or alters a trade. On any
    /// guardrail violation or completion error the commentary degrades to an
    /// empty string so the trading loop is never blocked.
    async fn cycle_commentary(
        &self,
        regime: &str,
        top_scores: &[(String, f64)],
        order_count: usize,
    ) -> String {
        // Guardrail check: this must stay within the advisory boundary.
        if let Err(violation) = authorize(&LlmAction::Explain) {
            tracing::warn!(error = %violation, "commentary skipped: guardrail violation");
            return String::new();
        }

        let top_symbols: Vec<&str> = top_scores
            .iter()
            .map(|(symbol, _)| symbol.as_str())
            .collect();
        let order_summary = format!("{order_count} order(s) proposed this cycle");
        let prompt = build_explanation_prompt(regime, &top_symbols, &order_summary);

        match self.llm.complete(&prompt).await {
            Ok(text) => text.trim().to_string(),
            Err(e) => {
                tracing::warn!(error = %e, "commentary generation failed");
                String::new()
            }
        }
    }

    /// Risk-check, quote, finalize, and execute one order.
    #[allow(clippy::too_many_arguments)]
    async fn process_order(
        &self,
        run_id: &str,
        intent: &OrderIntent,
        snapshot: &MarketSnapshot,
        executor: &dyn TwakExecutor,
        risk: &RiskEngine,
        portfolio: &mut PortfolioState,
        drawdown: &mut DrawdownTracker,
        events: &mut RuntimeEventLog,
    ) -> bool {
        let nav = portfolio.nav_usd();
        let ctx = build_risk_context(intent, snapshot, portfolio, drawdown, nav);
        events.append(
            run_id,
            AgentEvent::OrderProposed,
            json!({ "from": intent.from_symbol, "to": intent.to_symbol, "amount_usd": intent.amount_usd.to_string() }),
        );

        // Pre-trade gate (no quote yet).
        if let RiskDecision::Rejected { reasons } = risk.pre_trade(intent, &ctx) {
            tracing::info!(?reasons, to = %intent.to_symbol, "order rejected pre-trade");
            events.append(
                run_id,
                AgentEvent::RiskRejected,
                json!({ "stage": "pretrade", "reasons": reasons }),
            );
            return false;
        }

        // Quote from TWAK.
        let quote: SwapQuote = match executor.quote_swap(intent).await {
            Ok(q) => q,
            Err(e) => {
                tracing::warn!(error = %e, "quote failed; skipping order");
                events.append(
                    run_id,
                    AgentEvent::RiskRejected,
                    json!({ "stage": "quote", "error": e.to_string() }),
                );
                return false;
            }
        };
        events.append(
            run_id,
            AgentEvent::TwakQuoteReceived,
            json!({ "route": quote.route_id, "slippage_pct": quote.summary.slippage_pct.to_string() }),
        );

        // Final approval with the quote attached.
        let approved = match risk.approve(intent.clone(), &ctx, &quote.summary) {
            Ok(a) => a,
            Err(decision) => {
                let reasons = match &decision {
                    RiskDecision::Rejected { reasons } => reasons.clone(),
                    _ => vec!["rejected".into()],
                };
                tracing::info!(?reasons, "order rejected at final check");
                events.append(
                    run_id,
                    AgentEvent::RiskRejected,
                    json!({ "stage": "final", "reasons": reasons }),
                );
                return false;
            }
        };
        if matches!(approved.decision, RiskDecision::Clipped { .. }) {
            events.append(
                run_id,
                AgentEvent::RiskClipped,
                json!({ "amount_usd": approved.approved_amount_usd.to_string() }),
            );
        } else {
            events.append(
                run_id,
                AgentEvent::RiskApproved,
                json!({ "amount_usd": approved.approved_amount_usd.to_string() }),
            );
        }

        // Execute via TWAK.
        events.append(
            run_id,
            AgentEvent::TwakSwapSubmitted,
            json!({ "amount_usd": approved.approved_amount_usd.to_string() }),
        );
        let receipt = match executor.execute_swap(&approved).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(error = %e, "swap execution failed");
                events.append(
                    run_id,
                    AgentEvent::RiskRejected,
                    json!({ "stage": "execute", "error": e.to_string() }),
                );
                return false;
            }
        };
        events.append(
            run_id,
            AgentEvent::TxConfirmed,
            json!({ "tx_hash": receipt.tx_hash, "status": receipt.status, "block": receipt.block_number }),
        );
        tracing::info!(tx = %receipt.tx_hash, from = %intent.from_symbol, to = %intent.to_symbol, "swap confirmed");

        // Reconcile the book.
        let fill = build_fill(intent, snapshot, &quote, approved.approved_amount_usd);
        apply_fill(portfolio, &fill);
        let _ = drawdown; // drawdown is refreshed at the next cycle's mark step
        true
    }
}

struct RuntimeEventLog {
    memory: EventRepository,
    sqlite: Option<SqliteEventRepository>,
}

impl RuntimeEventLog {
    fn new(database_url: &str) -> Self {
        let sqlite = sqlite_path(database_url).and_then(|path| {
            if let Some(parent) = path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    tracing::warn!(path = %parent.display(), error = %e, "failed to create database directory");
                    return None;
                }
            }

            match SqliteEventRepository::open(&path) {
                Ok(repo) => {
                    tracing::info!(path = %path.display(), "SQLite event store enabled");
                    Some(repo)
                }
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "failed to open SQLite event store; using memory only");
                    None
                }
            }
        });

        Self {
            memory: EventRepository::new_memory(),
            sqlite,
        }
    }

    fn append(&mut self, run_id: impl Into<String>, event_type: AgentEvent, payload_json: Value) {
        let run_id = run_id.into();
        self.memory
            .append(run_id.clone(), event_type.clone(), payload_json.clone());

        if let Some(sqlite) = &self.sqlite {
            if let Err(e) = sqlite.append(run_id, event_type, payload_json) {
                tracing::warn!(error = %e, "failed to persist event to SQLite");
            }
        }
    }

    fn len(&self) -> usize {
        self.memory.all().len()
    }
}

fn sqlite_path(database_url: &str) -> Option<std::path::PathBuf> {
    database_url
        .strip_prefix("sqlite://")
        .filter(|path| !path.is_empty())
        .map(std::path::PathBuf::from)
}

/// Choose the market-data transport from config, then build the matching
/// [`CmcDataSource`].
///
/// - `cmc.use_mock` -> offline [`CmcTransport::Mock`].
/// - else `cmc.use_mcp` with `cmc.mcp_url` present -> [`CmcTransport::Mcp`].
/// - else `CMC_API_KEY` set -> [`CmcTransport::Rest`].
/// - else (paper only) fall back to the mock.
///
/// **Live mode never silently uses the mock.** If a live run cannot resolve a
/// real CMC transport (missing `CMC_API_KEY`/`mcp_url`, or `use_mock = true`),
/// or construction fails, this returns an error so the agent refuses to trade on
/// fake data. In paper mode it degrades to the offline mock as before.
fn build_data_source(s: &Settings) -> anyhow::Result<(Box<dyn CmcDataSource>, &'static str)> {
    let api_key = std::env::var("CMC_API_KEY").unwrap_or_default();
    let mcp_url = s.cmc.mcp_url.clone().unwrap_or_default();
    let live = s.app.is_live();

    let transport = if s.cmc.use_mock {
        if live {
            anyhow::bail!("live mode requires a real CMC source, but cmc.use_mock = true");
        }
        CmcTransport::Mock
    } else if s.cmc.use_mcp && !mcp_url.is_empty() {
        tracing::info!("using live CMC MCP data source");
        CmcTransport::Mcp
    } else if !api_key.is_empty() {
        tracing::info!("using live CMC REST data source");
        CmcTransport::Rest
    } else if live {
        anyhow::bail!(
            "live mode requires CMC_API_KEY (REST) or cmc.mcp_url (MCP); none configured"
        );
    } else {
        tracing::warn!("no CMC transport configured; using mock data source (paper)");
        CmcTransport::Mock
    };

    let label = cmc_transport_label(transport);

    match cmc_client::source_from(transport, api_key, mcp_url, s.cmc.request_timeout_ms) {
        Ok(source) => Ok((source, label)),
        Err(e) if live => Err(anyhow::anyhow!("failed to init live CMC data source: {e}")),
        Err(e) => {
            tracing::warn!(error = %e, "failed to init CMC data source; using mock (paper)");
            Ok((Box::new(MockCmcClient::new()), "mock"))
        }
    }
}

/// Stable operator-facing label for a CMC transport, used in startup logging and
/// the `AgentStarted` event so it's visible which source is live vs mock.
fn cmc_transport_label(transport: CmcTransport) -> &'static str {
    match transport {
        CmcTransport::Mock => "mock",
        CmcTransport::Mcp => "mcp",
        CmcTransport::Rest => "rest",
    }
}

/// Choose the TWAK execution transport from `twak.mode` and build the matching
/// [`TwakExecutor`].
///
/// `base_url` is taken from `twak.base_url`, falling back to the `TWAK_BASE_URL`
/// environment variable.
///
/// **Live mode never silently uses the mock.** A live run with `twak.mode =
/// "mock"`, an unknown mode, or a network transport without a `base_url` returns
/// an error rather than executing fake swaps. Paper mode degrades to the mock.
fn build_executor(s: &Settings) -> anyhow::Result<(Box<dyn TwakExecutor>, &'static str)> {
    let live = s.app.is_live();
    let transport = match s.twak.mode.as_str() {
        "rest" => TwakTransport::Rest,
        "mcp" => TwakTransport::Mcp,
        "cli" => TwakTransport::Cli,
        "mock" if live => {
            anyhow::bail!("live mode requires a real TWAK transport, but twak.mode = \"mock\"");
        }
        "mock" => TwakTransport::Mock,
        other if live => {
            anyhow::bail!("live mode: unknown twak.mode '{other}'");
        }
        other => {
            tracing::warn!(mode = %other, "unknown twak.mode; using mock executor (paper)");
            TwakTransport::Mock
        }
    };

    let env_base_url = std::env::var("TWAK_BASE_URL").ok();
    let base_url = s.twak.base_url.as_deref().or(env_base_url.as_deref());

    // A network transport in live mode must have a base_url, else executor_from
    // would silently hand back a mock.
    if live && matches!(transport, TwakTransport::Rest | TwakTransport::Mcp) && base_url.is_none() {
        anyhow::bail!(
            "live mode: twak.mode = \"{}\" requires twak.base_url or TWAK_BASE_URL",
            s.twak.mode
        );
    }

    let label = twak_transport_label(transport);
    Ok((
        twak_client::executor_from(transport, base_url, s.twak.autonomous),
        label,
    ))
}

/// Stable operator-facing label for a TWAK transport, used in startup logging
/// and the `AgentStarted` event so it's visible which executor is live vs mock.
fn twak_transport_label(transport: TwakTransport) -> &'static str {
    match transport {
        TwakTransport::Mock => "mock",
        TwakTransport::Rest => "rest",
        TwakTransport::Mcp => "mcp",
        TwakTransport::Cli => "cli",
    }
}

/// Immutable per-run metadata shared with each cycle's run-report write.
struct RunMeta {
    run_id: String,
    mode: String,
    wallet_address: String,
    starting_nav_usd: Decimal,
    policy_hash: String,
    agent_id: String,
    /// On-chain competition registration tx hash, once registered (live).
    registration_tx: Option<String>,
}

/// Compute the live regime-routed ensemble blend for the classified regime,
/// using the native `strategy-ensemble` crate over the embedded
/// `skills/ensemble.json`. Returns the per-skill blend weights for the current
/// regime (or `None` if the embedded config can't be parsed) so the live engine
/// surfaces exactly which Track-2 skills the ensemble would weight right now.
fn ensemble_routing(regime: &str) -> Option<serde_json::Value> {
    use strategy_ensemble::MarketRegime;
    let cfg = strategy_ensemble::EnsembleConfig::embedded().ok()?;
    let market_regime = match regime {
        "risk_on" => MarketRegime::RiskOn,
        "risk_off" => MarketRegime::RiskOff,
        "breakout" => MarketRegime::Breakout,
        _ => MarketRegime::Chop,
    };
    let weights = cfg.regime(market_regime)?.normalized();
    Some(json!({ "regime": regime, "skill_weights": weights }))
}

/// Persist the validated market snapshot to a per-run JSONL history file so the
/// analytics layer (python-lab notebooks, charts) has real market history to
/// work with. One JSON line per cycle at `data/snapshots/<run_id>.jsonl`
/// (override the base dir with `GUARDRAIL_SNAPSHOT_DIR`). Best-effort: any
/// filesystem/serialization error is logged and never interrupts the loop.
fn persist_snapshot(run_id: &str, snapshot: &MarketSnapshot) {
    let dir = std::env::var("GUARDRAIL_SNAPSHOT_DIR")
        .unwrap_or_else(|_| "data/snapshots".to_string());
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::warn!(error = %e, dir = %dir, "could not create snapshot dir");
        return;
    }
    let line = match serde_json::to_string(snapshot) {
        Ok(json) => json,
        Err(e) => {
            tracing::warn!(error = %e, "could not serialize snapshot");
            return;
        }
    };
    let path = format!("{dir}/{run_id}.jsonl");
    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut file) => {
            use std::io::Write;
            if let Err(e) = writeln!(file, "{line}") {
                tracing::warn!(error = %e, path = %path, "could not append snapshot");
            }
        }
        Err(e) => tracing::warn!(error = %e, path = %path, "could not open snapshot file"),
    }
}

/// Write the live run report to disk for the monitor sidecar and dashboard.
/// Path is `GUARDRAIL_REPORT` or `data/run_report.json`. Best-effort: failures
/// are logged, never fatal to the trading loop.
#[allow(clippy::too_many_arguments)]
fn write_run_report(
    meta: &RunMeta,
    portfolio: &PortfolioState,
    drawdown: &DrawdownTracker,
    regime: &str,
    events_count: usize,
    kill_switch: bool,
    commentary: &str,
) {
    let nav = portfolio.nav_usd();
    let positions: Vec<Value> = portfolio
        .risk_weights_pct()
        .into_iter()
        .map(|(symbol, w)| {
            json!({
                "symbol": symbol,
                "weight_pct": w.round_dp(2).to_string(),
                "value_usd": (w / Decimal::from(100) * nav).round_dp(2).to_string(),
            })
        })
        .collect();

    let mut report = json!({
        "run_id": meta.run_id,
        "mode": meta.mode,
        "updated_ms": now_ms(),
        "wallet_address": meta.wallet_address,
        "nav_usd": nav.round_dp(2).to_string(),
        "starting_nav_usd": meta.starting_nav_usd.to_string(),
        "total_drawdown_pct": drawdown.total_drawdown_pct(nav).round_dp(4).to_string(),
        "regime": regime,
        "kill_switch": kill_switch,
        "commentary": commentary,
        "positions": positions,
        "trades": [],
        "events": events_count,
        "policy_hash": meta.policy_hash,
        "agent_id": meta.agent_id,
    });
    if let Some(tx) = &meta.registration_tx {
        report["registration_tx"] = json!(tx);
    }

    let path = std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| "data/run_report.json".into());
    if let Some(parent) = std::path::Path::new(&path).parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            tracing::warn!(dir = %parent.display(), error = %e, "failed to create run-report dir; report may be lost");
        }
    }
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                tracing::warn!(path = %path, error = %e, "failed to write run report");
            }
        }
        Err(e) => tracing::warn!(error = %e, "failed to serialize run report"),
    }
}

/// Map a configured allocation-method string to its [`AllocationMethod`].
/// Unknown values fall back to `ScoreProportional`.
fn parse_allocation_method(method: &str) -> AllocationMethod {
    match method {
        "equal_weight" => AllocationMethod::EqualWeight,
        "score_proportional" => AllocationMethod::ScoreProportional,
        "inverse_volatility" => AllocationMethod::InverseVolatility,
        "risk_parity" => AllocationMethod::RiskParity,
        _ => AllocationMethod::ScoreProportional,
    }
}

/// Build the strategy config from runtime settings and the risk position cap.
fn build_strategy_config(s: &Settings, position_cap_pct: f64) -> StrategyConfig {
    StrategyConfig {
        max_positions: s.strategy.max_positions,
        min_score_to_enter: s.strategy.min_score_to_enter,
        min_score_to_hold: s.strategy.min_score_to_hold,
        rebalance_threshold_pct: to_f64(s.strategy.rebalance_threshold_pct),
        max_position_weight_pct: position_cap_pct,
        stop_loss_pct: s.strategy.stop_loss_pct,
        take_profit_pct: s.strategy.take_profit_pct,
        target_stable_reserve_pct: s.strategy.target_stable_reserve_pct,
        allocation_method: parse_allocation_method(&s.strategy.allocation_method),
        ..StrategyConfig::default()
    }
}

/// The non-stable asset with the strongest 24h return — the cold-start heartbeat target.
fn top_risk_symbol(snapshot: &MarketSnapshot) -> Option<String> {
    snapshot
        .assets
        .iter()
        .filter(|a| !a.asset.category.is_stable())
        .max_by(|a, b| {
            a.ret_24h
                .unwrap_or(Decimal::ZERO)
                .cmp(&b.ret_24h.unwrap_or(Decimal::ZERO))
        })
        .map(|a| a.asset.symbol.clone())
}

/// Build a minimal compliant heartbeat trade for the daily-activity requirement.
///
/// Prefers trimming the largest held position into the reserve (always within
/// balance and never increases concentration). Falls back to a small buy of the
/// strongest name when the book is all stables (cold start).
fn heartbeat_intent(
    portfolio: &PortfolioState,
    snapshot: &MarketSnapshot,
    pct: Decimal,
) -> Option<OrderIntent> {
    let nav = portfolio.nav_usd();
    if nav <= Decimal::ZERO {
        return None;
    }
    let amount = common::decimal::apply_pct(nav, pct);

    // Largest non-reserve holding, if any.
    let largest = portfolio
        .risk_weights_pct()
        .into_iter()
        .filter(|(_, w)| *w > Decimal::ZERO)
        .max_by(|a, b| a.1.cmp(&b.1))
        .map(|(sym, _)| sym);

    if let Some(symbol) = largest {
        return Some(OrderIntent::new(
            OrderSide::Sell,
            symbol,
            RESERVE_SYMBOL,
            amount,
            "daily-trade heartbeat (trim)",
        ));
    }
    let top = top_risk_symbol(snapshot)?;
    strategy_engine::daily_trade::heartbeat_order(&top, nav, pct)
}

/// Symbol -> price map for marking the book, with the stable reserve pinned at $1.
fn price_map(snapshot: &MarketSnapshot) -> HashMap<String, Decimal> {
    let mut prices = HashMap::new();
    prices.insert(RESERVE_SYMBOL.to_string(), Decimal::ONE);
    for a in &snapshot.assets {
        prices.insert(a.asset.symbol.clone(), a.price_usd);
    }
    prices
}

/// Price of a symbol from the snapshot; stables pin to $1.
fn price_of(symbol: &str, snapshot: &MarketSnapshot) -> Decimal {
    if symbol == RESERVE_SYMBOL {
        return Decimal::ONE;
    }
    snapshot
        .get(symbol)
        .map(|s| s.price_usd)
        .unwrap_or(Decimal::ONE)
}

/// Assemble the value-based risk context for one order.
fn build_risk_context(
    intent: &OrderIntent,
    snapshot: &MarketSnapshot,
    portfolio: &PortfolioState,
    drawdown: &DrawdownTracker,
    nav: Decimal,
) -> RiskContext {
    // The risk symbol is the side gaining exposure (buy) or being trimmed (sell).
    let (risk_symbol, projected_pct) = match intent.side {
        OrderSide::Buy => {
            let current = portfolio.weight_pct(&intent.to_symbol);
            let added = if nav > Decimal::ZERO {
                intent.amount_usd / nav * Decimal::from(100)
            } else {
                Decimal::ZERO
            };
            (intent.to_symbol.clone(), current + added)
        }
        OrderSide::Sell => (
            intent.from_symbol.clone(),
            portfolio.weight_pct(&intent.from_symbol),
        ),
    };

    let security_flags = snapshot
        .get(&risk_symbol)
        .map(|s| s.security_flags.clone())
        .unwrap_or_default();

    RiskContext {
        nav_usd: nav,
        stable_reserve_pct: portfolio.stable_reserve_pct(),
        total_drawdown_pct: drawdown.total_drawdown_pct(nav),
        daily_drawdown_pct: drawdown.daily_drawdown_pct(nav),
        target_position_pct: projected_pct,
        security_flags,
    }
}

/// Build a portfolio fill from an executed order and its quote.
fn build_fill(
    intent: &OrderIntent,
    snapshot: &MarketSnapshot,
    quote: &SwapQuote,
    amount_usd: Decimal,
) -> Fill {
    let from_price = price_of(&intent.from_symbol, snapshot);
    let to_price = price_of(&intent.to_symbol, snapshot);
    // Cost = quoted slippage on the notional plus a small fixed gas charge.
    let fee = (amount_usd * quote.summary.slippage_pct / Decimal::from(100)) + Decimal::new(35, 2);
    Fill {
        from_symbol: intent.from_symbol.clone(),
        to_symbol: intent.to_symbol.clone(),
        notional_usd: amount_usd,
        to_price_usd: to_price,
        from_price_usd: from_price,
        fee_usd: fee,
    }
}

#[cfg(test)]
mod tests {
    use super::{build_data_source, build_executor, sqlite_path};

    /// Load a committed config, resolving its path from the crate manifest dir so
    /// the test does not depend on the runner's working directory.
    fn cfg(rel: &str) -> common::Settings {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(rel);
        common::Settings::load(path.to_str().unwrap()).expect("load config")
    }

    #[test]
    fn parses_sqlite_database_url() {
        assert_eq!(
            sqlite_path("sqlite://data/guardrail_alpha.db").as_deref(),
            Some(std::path::Path::new("data/guardrail_alpha.db"))
        );
    }

    #[test]
    fn ignores_non_sqlite_database_url() {
        assert!(sqlite_path("postgres://localhost/db").is_none());
    }

    #[test]
    fn live_mode_refuses_mock_cmc() {
        // These bail before any env read, so the test is deterministic.
        let mut s = cfg("configs/production.toml");
        assert!(s.app.is_live(), "production config must be live mode");
        s.cmc.use_mock = true;
        assert!(
            build_data_source(&s).is_err(),
            "live mode must refuse a mock CMC source"
        );
    }

    #[test]
    fn live_mode_refuses_mock_twak() {
        let mut s = cfg("configs/production.toml");
        s.twak.mode = "mock".into();
        assert!(
            build_executor(&s).is_err(),
            "live mode must refuse a mock TWAK executor"
        );
    }

    #[test]
    fn paper_mode_allows_mock() {
        let s = cfg("configs/paper.toml");
        assert!(!s.app.is_live());
        assert!(build_data_source(&s).is_ok());
        assert!(build_executor(&s).is_ok());
    }
}
