//! Guardrail Alpha developer/admin CLI.
//!
//! Real subcommands:
//!   backtest  — run the strategy + risk pipeline over a synthetic path
//!   score     — show the current regime and per-asset alpha scores
//!   quote     — compute an AMM-style swap quote (impact + slippage)
//!   policy    — hash a policy file for on-chain proof
//!   register  — print the competition registration target
//!   kill-switch — emit a kill-switch trigger line

mod commands;
mod util;

use clap::{Parser, Subcommand};
use common::decimal::to_f64;
use common::Settings;
use rust_decimal::Decimal;
use strategy_engine::{CurrentAllocation, StrategyConfig};
use util::decimal_from_str;

const DEFAULT_UNIVERSE: &str = "configs/eligible_assets.bsc.json";

/// Path to the selectable strategy presets file.
const STRATEGY_PRESETS_PATH: &str = "configs/strategy_presets.json";

/// Default preset applied when `--preset` is not supplied.
const DEFAULT_PRESET: &str = "balanced";

/// Optional, leniently-parsed overrides for a named strategy preset. Each field
/// is applied to the base `StrategyConfig` only when present in the JSON.
#[derive(Debug, Default, Clone, serde::Deserialize)]
struct PresetOverrides {
    min_score_to_enter: Option<f64>,
    min_score_to_hold: Option<f64>,
    max_positions: Option<u32>,
    rebalance_threshold_pct: Option<f64>,
    target_stable_reserve_pct: Option<f64>,
}

/// Apply a named preset's overrides onto `cfg`, preserving `max_position_weight_pct`
/// (the policy cap). On a missing file/preset, prints a note and returns `cfg`
/// unchanged so callers fall back to current behavior.
fn apply_preset(mut cfg: StrategyConfig, preset: &str) -> StrategyConfig {
    let raw = match std::fs::read_to_string(STRATEGY_PRESETS_PATH) {
        Ok(raw) => raw,
        Err(_) => {
            println!("note: preset file '{STRATEGY_PRESETS_PATH}' not found; using default config");
            return cfg;
        }
    };
    let presets: std::collections::HashMap<String, PresetOverrides> =
        match serde_json::from_str(&raw) {
            Ok(presets) => presets,
            Err(e) => {
                println!(
                    "note: failed to parse '{STRATEGY_PRESETS_PATH}' ({e}); using default config"
                );
                return cfg;
            }
        };
    match presets.get(preset) {
        Some(overrides) => {
            if let Some(v) = overrides.min_score_to_enter {
                cfg.min_score_to_enter = v;
            }
            if let Some(v) = overrides.min_score_to_hold {
                cfg.min_score_to_hold = v;
            }
            if let Some(v) = overrides.max_positions {
                cfg.max_positions = v;
            }
            if let Some(v) = overrides.rebalance_threshold_pct {
                cfg.rebalance_threshold_pct = v;
            }
            if let Some(v) = overrides.target_stable_reserve_pct {
                cfg.target_stable_reserve_pct = v;
            }
            println!("active preset: {preset}");
        }
        None => {
            println!(
                "note: preset '{preset}' not found in '{STRATEGY_PRESETS_PATH}'; using default config"
            );
        }
    }
    cfg
}

#[derive(Debug, Parser)]
#[command(name = "guardrail", about = "Guardrail Alpha CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run a backtest of the live strategy over a synthetic market path.
    Backtest {
        #[arg(long, default_value = "configs/paper.toml")]
        config: String,
        #[arg(long, default_value_t = 60)]
        steps: u32,
        /// Strategy preset to apply (see configs/strategy_presets.json).
        #[arg(long, default_value = DEFAULT_PRESET)]
        preset: String,
    },
    /// Backtest all strategy presets side by side and print a comparison table.
    Compare {
        #[arg(long, default_value = "configs/paper.toml")]
        config: String,
        #[arg(long, default_value_t = 60)]
        steps: u32,
        #[arg(long, default_value_t = 60)]
        fear_greed: u32,
    },
    /// Show the current market regime and asset alpha scores.
    Score {
        #[arg(long, default_value = "configs/paper.toml")]
        config: String,
    },
    /// Compute a swap quote (price impact + slippage) for a notional.
    Quote {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: String,
    },
    /// Run a walk-forward analysis across sentiment-driven windows.
    WalkForward {
        #[arg(long, default_value = "configs/paper.toml")]
        config: String,
        #[arg(long, default_value_t = 6)]
        windows: u32,
        #[arg(long, default_value_t = 30)]
        steps: u32,
        /// Strategy preset to apply (see configs/strategy_presets.json).
        #[arg(long, default_value = DEFAULT_PRESET)]
        preset: String,
    },
    /// Print a live market table for the eligible universe via the CMC data path.
    Markets {
        #[arg(long, default_value = "configs/paper.toml")]
        config: String,
        #[arg(long)]
        live: bool,
    },
    /// Rank enabled assets by current attention needs.
    Watchlist {
        #[arg(long, default_value_t = 12)]
        limit: usize,
    },
    /// Show liquidity capacity and pool usage for eligible assets.
    Liquidity {
        #[arg(long, default_value = "configs/liquidity/liquidity_policy.json")]
        policy: String,
        #[arg(long, default_value_t = 12)]
        limit: usize,
    },
    /// Estimate gas and slippage cost for configured TWAK routes.
    Costs {
        #[arg(long, default_value = "configs/costs/bsc_execution_costs.json")]
        config: String,
        #[arg(long)]
        amount_usd: Option<String>,
    },
    /// Show daily execution budget and gas-runway status.
    Budget {
        #[arg(long, default_value = "configs/budgets/trading_budget.json")]
        config: String,
    },
    /// Compare current report weights with a fresh strategy target.
    Drift {
        #[arg(long, default_value = "configs/drift/drift_policy.json")]
        policy: String,
    },
    /// Compile configured natural-language mandates into policy hashes.
    Mandates {
        #[arg(long, default_value = "configs/mandates/strategy_mandates.json")]
        config: String,
    },
    /// Print judge/operator briefing claims and demo commands.
    Briefing {
        #[arg(long, default_value = "data/run_report.json")]
        report: String,
        #[arg(long, default_value = "configs/briefings/submission_briefing.json")]
        config: String,
    },
    /// Register the agent for the competition through TWAK (self-custody).
    Register {
        /// TWAK transport: mock (offline default), rest, mcp, or cli.
        #[arg(long, default_value = "mock")]
        transport: String,
        /// Base URL for the rest/mcp transports (or set TWAK_BASE_URL).
        #[arg(long)]
        base_url: Option<String>,
        /// Allow the executor to self-submit the registration transaction.
        #[arg(long, default_value_t = true)]
        autonomous: bool,
    },
    /// Print the agent's BNB identity and proof commitments as JSON.
    Identity {
        #[arg(long, default_value = "configs/paper.toml")]
        config: String,
    },
    /// Hash a policy file (SHA-256) for on-chain proof.
    Policy {
        #[command(subcommand)]
        command: PolicyCommand,
    },
    /// Emit a kill-switch trigger.
    KillSwitch {
        #[arg(long)]
        reason: Option<String>,
    },
    /// Render an offline Markdown run report from the agent's persisted state.
    Report {
        #[arg(long, default_value = "data/run_report.json")]
        report: String,
    },
    /// Compute classic technical indicators over a deterministic price series.
    Indicators {
        #[arg(long, default_value = "WBNB")]
        symbol: String,
        #[arg(long, default_value_t = 48)]
        steps: u32,
    },
    /// Track and compare named backtest experiments saved under data/experiments/.
    Experiment {
        #[command(subcommand)]
        command: ExperimentCommand,
    },
    /// Print a concise DoraHacks submission summary from the latest run.
    Submission {
        #[arg(long, default_value = "data/run_report.json")]
        report: String,
    },
    /// Classify the current market regime and show its sizing exposure.
    Regime {
        #[arg(long, default_value = "configs/paper.toml")]
        config: String,
    },
    /// Print a per-asset funding-rate proxy table over a synthetic snapshot.
    Funding {
        #[arg(long, default_value_t = 48)]
        steps: u32,
    },
    /// Preview target weights and trade intents without executing anything.
    Rebalance {
        #[arg(long, default_value = "configs/paper.toml")]
        config: String,
        #[arg(long, default_value = "data/run_report.json")]
        report: String,
        #[arg(long)]
        nav_usd: Option<String>,
        /// Strategy preset to apply (see configs/strategy_presets.json).
        #[arg(long, default_value = DEFAULT_PRESET)]
        preset: String,
    },
    /// Show current category exposure from the latest run report.
    Exposure {
        #[arg(long, default_value = "data/run_report.json")]
        report: String,
        #[arg(long, default_value = DEFAULT_UNIVERSE)]
        universe: String,
    },
    /// Select the current operator playbook from run state.
    Playbook {
        #[arg(long, default_value = "data/run_report.json")]
        report: String,
        #[arg(long, default_value = "configs/playbooks/operator_actions.json")]
        playbooks: String,
    },
    /// Apply configured market stress scenarios to the current report.
    Scenarios {
        #[arg(long, default_value = "data/run_report.json")]
        report: String,
        #[arg(long, default_value = DEFAULT_UNIVERSE)]
        universe: String,
        #[arg(long, default_value = "configs/scenarios/market_stress.json")]
        scenarios: String,
    },
    /// Show hackathon prize/category evidence map.
    Prizes {
        #[arg(long, default_value = "configs/submission/prize_map.json")]
        config: String,
        #[arg(long, default_value = "data/run_report.json")]
        report: String,
    },
    /// Show self-custody wallet and spender control status.
    WalletControls {
        #[arg(long, default_value = "configs/wallet/wallet_controls.json")]
        config: String,
    },
    /// Evaluate current positions against configured exit triggers.
    ExitTriggers {
        #[arg(long, default_value = "configs/exits/exit_policy.json")]
        policy: String,
    },
    /// Inventory submission artifacts and declared operator routes.
    AuditManifest {
        #[arg(long, default_value = "configs/audit/export_manifest.json")]
        config: String,
    },
    /// Show BNB Agent SDK module and contract mapping evidence.
    BnbSdk {
        #[arg(long, default_value = "configs/bnb/bnb_agent_sdk_map.json")]
        config: String,
    },
    /// Show ERC-8183 commerce/provider readiness mapping.
    Commerce {
        #[arg(long, default_value = "configs/bnb/erc8183_commerce.json")]
        config: String,
    },
    /// Show x402 and EIP-712 signing policy controls.
    SigningPolicy {
        #[arg(long, default_value = "configs/x402/signing_policy.json")]
        config: String,
    },
    /// Show Track-1 daily-trade heartbeat status and planned tiny order.
    Heartbeat {
        #[arg(long, default_value = "configs/heartbeat/daily_trade.json")]
        config: String,
    },
    /// Show judge-facing weighted submission scorecard.
    Scorecard {
        #[arg(long, default_value = "configs/submission/scorecard.json")]
        config: String,
    },
    /// Inspect the product-owned BNB Agent SDK integration tree.
    SdkCatalog,
    /// Show ERC-8183 provider service offerings backed by Guardrail routes.
    AgentServices {
        #[arg(long, default_value = "configs/bnb/agent_services.json")]
        config: String,
    },
    /// Render the ERC-8004-style Guardrail agent card.
    AgentCard {
        #[arg(long, default_value = "configs/bnb/agent_card.json")]
        config: String,
    },
    /// Simulate an ERC-8183 job lifecycle against a Guardrail service.
    JobSimulator {
        #[arg(long, default_value = "configs/bnb/job_simulator.json")]
        config: String,
    },
    /// Compute a technical indicator over a close-price series (ta-signals).
    Ta {
        #[arg(long, default_value = "sma")]
        indicator: String,
        #[arg(long)]
        series: String,
        #[arg(long, default_value_t = 14)]
        period: usize,
    },
    /// Estimate the all-in cost of a swap (fee-model).
    Fees {
        #[arg(long, default_value_t = 10_000.0)]
        notional: f64,
        #[arg(long, default_value_t = 5.0)]
        quantity: f64,
        #[arg(long, default_value = "buy")]
        side: String,
    },
    /// Compute a position size (position-sizer).
    Size {
        #[arg(long, default_value = "kelly")]
        method: String,
        #[arg(long, default_value_t = 10_000.0)]
        capital: f64,
        #[arg(long, default_value_t = 0.55)]
        win_prob: f64,
        #[arg(long, default_value_t = 1.0)]
        odds: f64,
    },
    /// Run the order-book matching engine over an order spec (orderbook).
    Book {
        #[arg(long, default_value = "s,limit,101,5;b,limit,99,5;b,market,,6")]
        orders: String,
    },
    /// Average-cost PnL attribution from a fill spec (pnl-attribution).
    Pnl {
        #[arg(long, default_value = "CAKE,buy,10,2;CAKE,sell,4,3;WBNB,buy,5,600")]
        fills: String,
        #[arg(long, default_value = "")]
        marks: String,
    },
}

#[derive(Debug, Subcommand)]
enum ExperimentCommand {
    /// Run a backtest and persist it as a named experiment.
    Run {
        /// Name used as the experiment identifier and file name.
        #[arg(long)]
        tag: String,
        #[arg(long, default_value = "configs/paper.toml")]
        config: String,
        #[arg(long, default_value_t = 60)]
        steps: u32,
        #[arg(long, default_value_t = 60)]
        fear_greed: u32,
        /// Strategy preset to apply (see configs/strategy_presets.json).
        #[arg(long, default_value = DEFAULT_PRESET)]
        preset: String,
    },
    /// List all saved experiments with their key metrics.
    List,
    /// Print a Markdown table comparing all saved experiments.
    Compare,
}

#[derive(Debug, Subcommand)]
enum PolicyCommand {
    /// Hash a policy JSON file (SHA-256).
    Hash { path: String },
    /// Compile a natural-language mandate into a validated policy + hash.
    Compile { mandate: String },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Backtest {
            config,
            steps,
            preset,
        } => commands::backtest::run_backtest(&config, steps, &preset)?,
        Commands::Compare {
            config,
            steps,
            fear_greed,
        } => commands::backtest::run_compare(&config, steps, fear_greed)?,
        Commands::Score { config } => commands::backtest::run_score(&config)?,
        Commands::WalkForward {
            config,
            windows,
            steps,
            preset,
        } => commands::backtest::run_walk_forward(&config, windows, steps, &preset)?,
        Commands::Quote { from, to, amount } => commands::market::run_quote(&from, &to, &amount)?,
        Commands::Markets { config, live } => commands::market::run_markets(&config, live)?,
        Commands::Watchlist { limit } => commands::market::run_watchlist(limit)?,
        Commands::Liquidity { policy, limit } => commands::market::run_liquidity(&policy, limit)?,
        Commands::Costs { config, amount_usd } => {
            commands::market::run_costs(&config, amount_usd.as_deref())?
        }
        Commands::Budget { config } => commands::portfolio::run_budget(&config)?,
        Commands::Drift { policy } => commands::portfolio::run_drift(&policy)?,
        Commands::Mandates { config } => commands::portfolio::run_mandates(&config)?,
        Commands::Briefing { report, config } => {
            commands::reporting::run_briefing(&report, &config)?
        }
        Commands::Register {
            transport,
            base_url,
            autonomous,
        } => commands::identity::run_register(&transport, base_url.as_deref(), autonomous)?,
        Commands::Policy {
            command: PolicyCommand::Hash { path },
        } => {
            let bytes = std::fs::read(&path)?;
            println!("{}", policy_compiler::policy_hash(&bytes));
        }
        Commands::Policy {
            command: PolicyCommand::Compile { mandate },
        } => {
            let compiled = policy_compiler::compile_mandate(&mandate)?;
            println!("policy_hash: {}\n", compiled.hash);
            println!(
                "{}",
                policy_compiler::compiler::policy_to_json(&compiled.policy)?
            );
        }
        Commands::Identity { config } => commands::identity::run_identity(&config)?,
        Commands::Report { report } => commands::reporting::run_report(&report)?,
        Commands::Indicators { symbol, steps } => commands::market::run_indicators(&symbol, steps)?,
        Commands::Experiment {
            command:
                ExperimentCommand::Run {
                    tag,
                    config,
                    steps,
                    fear_greed,
                    preset,
                },
        } => commands::experiment::run_experiment_run(&tag, &config, steps, fear_greed, &preset)?,
        Commands::Experiment {
            command: ExperimentCommand::List,
        } => commands::experiment::run_experiment_list()?,
        Commands::Experiment {
            command: ExperimentCommand::Compare,
        } => commands::experiment::run_experiment_compare()?,
        Commands::Submission { report } => commands::reporting::run_submission(&report)?,
        Commands::Regime { config } => commands::backtest::run_regime(&config)?,
        Commands::Funding { steps } => commands::backtest::run_funding(steps)?,
        Commands::Rebalance {
            config,
            report,
            nav_usd,
            preset,
        } => commands::portfolio::run_rebalance(&config, &report, nav_usd.as_deref(), &preset)?,
        Commands::Exposure { report, universe } => {
            commands::portfolio::run_exposure(&report, &universe)?
        }
        Commands::Playbook { report, playbooks } => {
            commands::portfolio::run_playbook(&report, &playbooks)?
        }
        Commands::Scenarios {
            report,
            universe,
            scenarios,
        } => commands::portfolio::run_scenarios(&report, &universe, &scenarios)?,
        Commands::Prizes { config, report } => commands::reporting::run_prizes(&config, &report)?,
        Commands::WalletControls { config } => commands::identity::run_wallet_controls(&config)?,
        Commands::ExitTriggers { policy } => commands::identity::run_exit_triggers(&policy)?,
        Commands::AuditManifest { config } => commands::reporting::run_audit_manifest(&config)?,
        Commands::BnbSdk { config } => commands::commerce::run_bnb_sdk(&config)?,
        Commands::Commerce { config } => commands::commerce::run_commerce(&config)?,
        Commands::SigningPolicy { config } => commands::commerce::run_signing_policy(&config)?,
        Commands::Heartbeat { config } => commands::identity::run_heartbeat(&config)?,
        Commands::Scorecard { config } => commands::agent_surface::run_scorecard(&config)?,
        Commands::SdkCatalog => commands::agent_surface::run_sdk_catalog()?,
        Commands::AgentServices { config } => commands::agent_surface::run_agent_services(&config)?,
        Commands::AgentCard { config } => commands::agent_surface::run_agent_card(&config)?,
        Commands::JobSimulator { config } => commands::agent_surface::run_job_simulator(&config)?,
        Commands::Ta { indicator, series, period } => commands::quant::run_ta(&indicator, &series, period)?,
        Commands::Fees { notional, quantity, side } => commands::quant::run_fees(notional, quantity, &side)?,
        Commands::Size { method, capital, win_prob, odds } => {
            commands::quant::run_size(&method, capital, win_prob, odds)?
        }
        Commands::Book { orders } => commands::quant::run_book(&orders)?,
        Commands::Pnl { fills, marks } => commands::quant::run_pnl(&fills, &marks)?,
        Commands::KillSwitch { reason } => {
            println!(
                "kill_switch_triggered reason={}",
                reason.unwrap_or_else(|| "manual".into())
            );
        }
    }
    Ok(())
}

/// Build a strategy config from settings, capping positions just under the risk
/// policy limit so targets are not auto-rejected.
fn strategy_config(settings: &Settings, position_cap_pct: f64) -> StrategyConfig {
    StrategyConfig {
        max_positions: settings.strategy.max_positions,
        min_score_to_enter: settings.strategy.min_score_to_enter,
        min_score_to_hold: settings.strategy.min_score_to_hold,
        rebalance_threshold_pct: to_f64(settings.strategy.rebalance_threshold_pct),
        max_position_weight_pct: position_cap_pct,
        ..StrategyConfig::default()
    }
}

/// Number of synthetic steps used to warm the price series before snapshotting.
const SNAPSHOT_WARMUP_STEPS: u32 = 5;

/// Fear/greed level used when warming the deterministic snapshot path.
const SNAPSHOT_FEAR_GREED: u32 = 60;

/// Build a deterministic, warmed market snapshot for the enabled universe.
///
/// Mirrors the warm-up `run_score` performs: it seeds each non-stable asset at
/// its synthetic initial price, evolves the path for a few steps so momentum is
/// meaningful, then builds a `MarketSnapshot`. Stable assets are pinned to $1.
fn build_warmed_snapshot(assets: &[common::Asset], warmup: u32) -> market_data::MarketSnapshot {
    use std::collections::HashMap;
    let mut prices: HashMap<String, Decimal> = HashMap::new();
    for a in assets {
        if a.category.is_stable() {
            prices.insert(a.symbol.clone(), Decimal::ONE);
        } else {
            prices.insert(
                a.symbol.clone(),
                backtester::synthetic::initial_price(&a.symbol),
            );
        }
    }
    for step in 0..warmup {
        for a in assets {
            if a.category.is_stable() {
                continue;
            }
            let r =
                backtester::synthetic::step_return_24h_pct(&a.symbol, step, SNAPSHOT_FEAR_GREED);
            if let Some(p) = prices.get_mut(&a.symbol) {
                *p *= Decimal::ONE + r / Decimal::from(100);
            }
        }
    }
    backtester::synthetic::build_snapshot(assets, &prices, warmup, SNAPSHOT_FEAR_GREED)
}

fn read_json_report(path: &str) -> Option<serde_json::Value> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn allocation_from_report(report: &serde_json::Value) -> CurrentAllocation {
    let mut current = CurrentAllocation::new();
    if let Some(positions) = report
        .get("positions")
        .and_then(serde_json::Value::as_array)
    {
        for position in positions {
            let Some(symbol) = position.get("symbol").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let Some(weight) = position
                .get("weight_pct")
                .and_then(serde_json::Value::as_str)
                .and_then(decimal_from_str)
            else {
                continue;
            };
            current = current.with_weight(symbol, weight);
        }
    }
    current
}

fn category_map_from_universe(
    universe: &serde_json::Value,
) -> std::collections::BTreeMap<String, String> {
    let mut map = std::collections::BTreeMap::new();
    if let Some(assets) = universe.as_array() {
        for asset in assets {
            let Some(symbol) = asset.get("symbol").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let Some(category) = asset.get("category").and_then(serde_json::Value::as_str) else {
                continue;
            };
            map.insert(symbol.to_string(), category.to_string());
        }
    }
    map
}

/// Default agent wallet used when `AGENT_WALLET` is not set in the environment.
const DEFAULT_AGENT_WALLET: &str = "0xA9e5C0FfEe0000000000000000000000000A1b2C3";
