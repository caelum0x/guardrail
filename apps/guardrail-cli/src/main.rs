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
use common::constants::COMPETITION_CONTRACT;
use common::decimal::to_f64;
use common::Settings;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::str::FromStr;
use strategy_engine::{CurrentAllocation, StrategyConfig, StrategyEngine};
use util::{
    cost_bps, decimal_from_str, decimal_value, gas_cost_usd, json_decimal_field, json_decimal_or,
    json_f64, json_num_fmt, json_str, scenario_shock_map,
};

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
        } => run_backtest(&config, steps, &preset)?,
        Commands::Compare {
            config,
            steps,
            fear_greed,
        } => run_compare(&config, steps, fear_greed)?,
        Commands::Score { config } => run_score(&config)?,
        Commands::WalkForward {
            config,
            windows,
            steps,
            preset,
        } => run_walk_forward(&config, windows, steps, &preset)?,
        Commands::Quote { from, to, amount } => run_quote(&from, &to, &amount)?,
        Commands::Markets { config, live } => run_markets(&config, live)?,
        Commands::Watchlist { limit } => run_watchlist(limit)?,
        Commands::Liquidity { policy, limit } => run_liquidity(&policy, limit)?,
        Commands::Costs { config, amount_usd } => run_costs(&config, amount_usd.as_deref())?,
        Commands::Budget { config } => run_budget(&config)?,
        Commands::Drift { policy } => run_drift(&policy)?,
        Commands::Mandates { config } => run_mandates(&config)?,
        Commands::Briefing { report, config } => run_briefing(&report, &config)?,
        Commands::Register {
            transport,
            base_url,
            autonomous,
        } => run_register(&transport, base_url.as_deref(), autonomous)?,
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
        Commands::Identity { config } => run_identity(&config)?,
        Commands::Report { report } => run_report(&report)?,
        Commands::Indicators { symbol, steps } => run_indicators(&symbol, steps)?,
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
        Commands::Submission { report } => run_submission(&report)?,
        Commands::Regime { config } => run_regime(&config)?,
        Commands::Funding { steps } => run_funding(steps)?,
        Commands::Rebalance {
            config,
            report,
            nav_usd,
            preset,
        } => run_rebalance(&config, &report, nav_usd.as_deref(), &preset)?,
        Commands::Exposure { report, universe } => run_exposure(&report, &universe)?,
        Commands::Playbook { report, playbooks } => run_playbook(&report, &playbooks)?,
        Commands::Scenarios {
            report,
            universe,
            scenarios,
        } => run_scenarios(&report, &universe, &scenarios)?,
        Commands::Prizes { config, report } => run_prizes(&config, &report)?,
        Commands::WalletControls { config } => run_wallet_controls(&config)?,
        Commands::ExitTriggers { policy } => run_exit_triggers(&policy)?,
        Commands::AuditManifest { config } => run_audit_manifest(&config)?,
        Commands::BnbSdk { config } => run_bnb_sdk(&config)?,
        Commands::Commerce { config } => run_commerce(&config)?,
        Commands::SigningPolicy { config } => run_signing_policy(&config)?,
        Commands::Heartbeat { config } => run_heartbeat(&config)?,
        Commands::Scorecard { config } => commands::agent_surface::run_scorecard(&config)?,
        Commands::SdkCatalog => commands::agent_surface::run_sdk_catalog()?,
        Commands::AgentServices { config } => commands::agent_surface::run_agent_services(&config)?,
        Commands::AgentCard { config } => commands::agent_surface::run_agent_card(&config)?,
        Commands::JobSimulator { config } => commands::agent_surface::run_job_simulator(&config)?,
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

fn run_backtest(config: &str, steps: u32, preset: &str) -> anyhow::Result<()> {
    let settings = Settings::load(config)?;
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let policy_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let policy = risk_engine::RiskPolicy::from_json_str(&policy_raw)?;
    let cap = (to_f64(policy.max_position_pct) - 1.0).max(1.0);
    let strat_cfg = apply_preset(strategy_config(&settings, cap), preset);

    let cfg = backtester::BacktestConfig {
        steps,
        ..Default::default()
    };
    let run = backtester::run_backtest(&universe, policy, strat_cfg, cfg);
    println!("{}", backtester::report::markdown(&run));
    Ok(())
}

/// Presets compared side by side, in increasing-risk order.
const COMPARE_PRESETS: [&str; 3] = ["conservative", "balanced", "aggressive"];

/// A single row of the preset comparison table.
struct CompareRow {
    preset: String,
    return_pct: f64,
    benchmark_pct: f64,
    excess_pct: f64,
    max_drawdown_pct: f64,
    calmar_ratio: f64,
    trade_count: u64,
}

fn run_compare(config: &str, steps: u32, fear_greed: u32) -> anyhow::Result<()> {
    let settings = Settings::load(config)?;
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let policy_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let policy = risk_engine::RiskPolicy::from_json_str(&policy_raw)?;
    let cap = (to_f64(policy.max_position_pct) - 1.0).max(1.0);

    let rows: Vec<CompareRow> = COMPARE_PRESETS
        .iter()
        .map(|preset| {
            let strat_cfg = apply_preset(strategy_config(&settings, cap), preset);
            let cfg = backtester::BacktestConfig {
                steps,
                starting_usd: Decimal::from(10_000),
                fear_greed,
            };
            let run = backtester::run_backtest(&universe, policy.clone(), strat_cfg, cfg);
            CompareRow {
                preset: (*preset).to_string(),
                return_pct: to_f64(run.metrics.total_return_pct),
                benchmark_pct: to_f64(run.benchmark_return_pct),
                excess_pct: to_f64(run.excess_return_pct),
                max_drawdown_pct: to_f64(run.metrics.max_drawdown_pct),
                calmar_ratio: to_f64(run.metrics.calmar_ratio),
                trade_count: run.metrics.trade_count,
            }
        })
        .collect();

    println!("# Strategy Preset Comparison");
    println!();
    println!("steps: {steps} · fear/greed: {fear_greed} · starting: $10,000");
    println!();
    println!("| Preset | Return % | Buy&Hold % | Excess % | Max DD % | Calmar | Trades |");
    println!("|:-------|---------:|-----------:|---------:|---------:|-------:|-------:|");
    for row in &rows {
        println!(
            "| {:<12} | {:>8.2} | {:>10.2} | {:>8.2} | {:>8.2} | {:>6.2} | {:>6} |",
            row.preset,
            row.return_pct,
            row.benchmark_pct,
            row.excess_pct,
            row.max_drawdown_pct,
            row.calmar_ratio,
            row.trade_count,
        );
    }
    Ok(())
}

fn run_walk_forward(config: &str, windows: u32, steps: u32, preset: &str) -> anyhow::Result<()> {
    let settings = Settings::load(config)?;
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let policy_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let policy = risk_engine::RiskPolicy::from_json_str(&policy_raw)?;
    let cap = (to_f64(policy.max_position_pct) - 1.0).max(1.0);
    let strat_cfg = apply_preset(strategy_config(&settings, cap), preset);

    let cfg = backtester::WalkForwardConfig {
        windows,
        steps_per_window: steps,
        fear_greed_path: vec![25, 40, 55, 70, 85, 60],
    };
    let report = backtester::walk_forward(&universe, policy, strat_cfg, cfg);

    println!("# Walk-Forward Analysis");
    println!();
    println!("windows: {windows} · steps/window: {steps}");
    println!();
    println!("| Window | Fear/Greed | Return % | Benchmark % | Excess % | Max DD % | Trades |");
    println!("|-------:|-----------:|---------:|------------:|---------:|---------:|-------:|");
    for w in &report.windows {
        println!(
            "| {:>6} | {:>10} | {:>8.2} | {:>11.2} | {:>8.2} | {:>8.2} | {:>6} |",
            w.window,
            w.fear_greed,
            to_f64(w.total_return_pct),
            to_f64(w.benchmark_return_pct),
            to_f64(w.excess_return_pct),
            to_f64(w.max_drawdown_pct),
            w.trades,
        );
    }
    println!();
    println!("## Aggregate");
    println!();
    println!("- mean excess: {:.2}%", to_f64(report.mean_excess_pct));
    println!(
        "- worst drawdown: {:.2}%",
        to_f64(report.worst_drawdown_pct)
    );
    println!(
        "- positive windows: {}/{}",
        report.positive_windows,
        report.windows.len()
    );
    Ok(())
}

fn run_score(config: &str) -> anyhow::Result<()> {
    let settings = Settings::load(config)?;
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let assets = universe.enabled_assets();

    // Warm up a short synthetic path so momentum is meaningful, then snapshot.
    use std::collections::HashMap;
    let mut prices: HashMap<String, Decimal> = HashMap::new();
    for a in &assets {
        if !a.category.is_stable() {
            prices.insert(
                a.symbol.clone(),
                backtester::synthetic::initial_price(&a.symbol),
            );
        } else {
            prices.insert(a.symbol.clone(), Decimal::ONE);
        }
    }
    let warmup = 5u32;
    for step in 0..warmup {
        for a in &assets {
            if a.category.is_stable() {
                continue;
            }
            let r = backtester::synthetic::step_return_24h_pct(&a.symbol, step, 60);
            if let Some(p) = prices.get_mut(&a.symbol) {
                *p *= Decimal::ONE + r / Decimal::from(100);
            }
        }
    }
    let snapshot = backtester::synthetic::build_snapshot(&assets, &prices, warmup, 60);

    let cap = (to_f64(risk_engine::RiskPolicy::default().max_position_pct) - 1.0).max(1.0);
    let strategy = StrategyEngine::new(strategy_config(&settings, cap));
    let decision = strategy.decide(&snapshot, &CurrentAllocation::new(), Decimal::from(10_000));

    println!("regime: {}", decision.regime.as_str());
    println!("{}", decision.explanation.headline);
    println!("\ntop alpha scores:");
    for (symbol, score) in &decision.explanation.top_scores {
        println!("  {symbol:<8} {score:.3}");
    }
    println!("\ntarget portfolio:");
    for t in &decision.target_positions {
        println!("  {:<8} {}%", t.symbol, t.weight_pct);
    }
    Ok(())
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

/// Classify the current synthetic market regime and print its inputs + exposure.
fn run_regime(config: &str) -> anyhow::Result<()> {
    let _settings = Settings::load(config)?;
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let assets = universe.enabled_assets();

    let snapshot = build_warmed_snapshot(&assets, SNAPSHOT_WARMUP_STEPS);
    let inputs = market_data::RegimeInputs::from_snapshot(&snapshot);
    let regime = strategy_engine::regime::classify(&inputs);

    println!("regime: {}", regime.as_str());
    println!("exposure multiplier: {:.2}", regime.exposure_multiplier());
    println!();
    println!("inputs:");
    println!("  fear_greed         : {}", inputs.fear_greed);
    println!("  breadth %          : {:.2}", to_f64(inputs.breadth_pct));
    println!(
        "  btc_dominance %    : {:.2}",
        to_f64(inputs.btc_dominance_pct)
    );
    println!(
        "  median 24h return %: {:.2}",
        to_f64(inputs.median_24h_return)
    );
    Ok(())
}

/// Funding-rate proxy bounds.
const FUNDING_PROXY_MIN: f64 = -1.0;
const FUNDING_PROXY_MAX: f64 = 1.0;

/// Print a per-asset funding-rate proxy table over a warmed synthetic snapshot.
///
/// The proxy is `ret_24h/24 + (volatility_1h - 3) * 0.01`, clamped to [-1, 1],
/// computed for each non-stable asset. It is a deterministic stand-in for a
/// perpetual funding rate while paper mode stays offline.
fn run_funding(steps: u32) -> anyhow::Result<()> {
    if steps == 0 {
        anyhow::bail!("steps must be greater than 0");
    }

    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let assets = universe.enabled_assets();
    let snapshot = build_warmed_snapshot(&assets, steps);

    println!("# Funding-Rate Proxy");
    println!();
    println!("steps: {steps} · fear/greed: {SNAPSHOT_FEAR_GREED}");
    println!();
    println!("{:<8} | {:>8} | {:>14}", "SYMBOL", "24H%", "FUNDING_PROXY");
    println!("{}", "-".repeat(36));

    for a in &snapshot.assets {
        if a.asset.category.is_stable() {
            continue;
        }
        let ret_24h = a.ret_24h.map(to_f64).unwrap_or(0.0);
        let vol_1h = a.volatility_1h.map(to_f64).unwrap_or(0.0);
        let proxy =
            (ret_24h / 24.0 + (vol_1h - 3.0) * 0.01).clamp(FUNDING_PROXY_MIN, FUNDING_PROXY_MAX);
        println!("{:<8} | {:>8.2} | {:>14.4}", a.asset.symbol, ret_24h, proxy);
    }
    Ok(())
}

/// Preview the strategy's next target book and order intents.
fn run_rebalance(
    config: &str,
    report_path: &str,
    nav_override: Option<&str>,
    preset: &str,
) -> anyhow::Result<()> {
    let settings = Settings::load(config)?;
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let policy_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let policy = risk_engine::RiskPolicy::from_json_str(&policy_raw)?;
    let cap = (to_f64(policy.max_position_pct) - 1.0).max(1.0);
    let strat_cfg = apply_preset(strategy_config(&settings, cap), preset);

    let report = read_json_report(report_path).unwrap_or_else(|| serde_json::json!({}));
    let nav_usd = nav_override
        .and_then(decimal_from_str)
        .or_else(|| {
            report
                .get("nav_usd")
                .and_then(serde_json::Value::as_str)
                .and_then(decimal_from_str)
        })
        .unwrap_or_else(|| Decimal::from(10_000));
    let current = allocation_from_report(&report);
    let assets = universe.enabled_assets();
    let snapshot = build_warmed_snapshot(&assets, SNAPSHOT_WARMUP_STEPS);
    let decision = StrategyEngine::new(strat_cfg.clone()).decide(&snapshot, &current, nav_usd);

    println!("# Rebalance Preview");
    println!();
    println!("config: {config}");
    println!("report: {report_path}");
    println!("preset: {preset}");
    println!("nav_usd: {}", nav_usd.round_dp(2));
    println!("regime: {}", decision.regime.as_str());
    println!(
        "exposure multiplier: {:.2}",
        decision.regime.exposure_multiplier()
    );
    println!(
        "threshold: {:.2}% · max positions: {} · position cap: {:.2}%",
        strat_cfg.rebalance_threshold_pct,
        strat_cfg.max_positions,
        strat_cfg.max_position_weight_pct
    );
    println!();
    println!("{}", decision.explanation.headline);
    println!();
    println!("| Symbol | Current % | Target % | Delta % |");
    println!("|:-------|----------:|---------:|--------:|");
    for target in &decision.target_positions {
        let current_w = current.weight(&target.symbol);
        let delta = target.weight_pct - current_w;
        println!(
            "| {:<6} | {:>9.2} | {:>8.2} | {:>7.2} |",
            target.symbol, current_w, target.weight_pct, delta
        );
    }
    println!();
    if decision.proposed_orders.is_empty() {
        println!("No orders proposed; current allocation is within threshold.");
    } else {
        println!("| Side | From | To | Amount USD | Reason |");
        println!("|:-----|:-----|:---|-----------:|:-------|");
        for order in &decision.proposed_orders {
            println!(
                "| {:?} | {} | {} | {:>10.2} | {} |",
                order.side, order.from_symbol, order.to_symbol, order.amount_usd, order.reason
            );
        }
        println!();
        println!("preview_only: true; orders still require risk gate, TWAK quote, and execution.");
    }
    Ok(())
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

fn run_exposure(report_path: &str, universe_path: &str) -> anyhow::Result<()> {
    let report = read_json_report(report_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read report {report_path}"))?;
    let universe = read_json_report(universe_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read universe {universe_path}"))?;
    let categories = category_map_from_universe(&universe);
    let positions = report
        .get("positions")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut category_rows: std::collections::BTreeMap<String, (Decimal, Decimal, usize)> =
        std::collections::BTreeMap::new();
    let mut weights = Vec::new();
    let mut largest_symbol = String::from("-");
    let mut largest_weight = Decimal::ZERO;

    for position in &positions {
        let symbol = position
            .get("symbol")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("UNKNOWN");
        let value = json_decimal_field(position, "value_usd");
        let weight = json_decimal_field(position, "weight_pct");
        let category = categories
            .get(symbol)
            .cloned()
            .unwrap_or_else(|| "uncategorized".to_string());
        let entry = category_rows
            .entry(category)
            .or_insert((Decimal::ZERO, Decimal::ZERO, 0));
        entry.0 += value;
        entry.1 += weight;
        entry.2 += 1;
        weights.push(weight);
        if weight > largest_weight {
            largest_symbol = symbol.to_string();
            largest_weight = weight;
        }
    }

    weights.sort_by(|a, b| b.cmp(a));
    let top3: Decimal = weights.iter().take(3).copied().sum();
    let stable = category_rows
        .get("stable")
        .map(|(_, weight, _)| *weight)
        .unwrap_or(Decimal::ZERO);
    let total: Decimal = weights.iter().copied().sum();
    let risk = (total - stable).max(Decimal::ZERO);

    println!("# Exposure");
    println!();
    println!("report: {report_path}");
    println!("universe: {universe_path}");
    println!(
        "nav_usd: {}",
        report
            .get("nav_usd")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-")
    );
    println!("positions: {}", positions.len());
    println!("largest: {largest_symbol} {:.2}%", largest_weight);
    println!("top3_weight: {:.2}%", top3);
    println!("risk_weight: {:.2}% · stable_weight: {:.2}%", risk, stable);
    println!();
    println!("| Category | Positions | Weight % | Value USD |");
    println!("|:---------|----------:|---------:|----------:|");
    for (category, (value, weight, count)) in category_rows {
        println!(
            "| {:<14} | {:>9} | {:>8.2} | {:>9.2} |",
            category, count, weight, value
        );
    }
    Ok(())
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

fn run_playbook(report_path: &str, playbooks_path: &str) -> anyhow::Result<()> {
    let report = read_json_report(report_path);
    let playbooks = read_json_report(playbooks_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read playbooks {playbooks_path}"))?;
    let kill_switch = report
        .as_ref()
        .and_then(|value| value.get("kill_switch"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let active_id = if kill_switch {
        "kill_switch"
    } else if report.is_none() {
        "bootstrap"
    } else {
        "ready"
    };
    let active = playbooks
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("id").and_then(serde_json::Value::as_str) == Some(active_id))
        })
        .ok_or_else(|| anyhow::anyhow!("playbook '{active_id}' not found"))?;

    println!("# Operator Playbook");
    println!();
    println!("active_id: {active_id}");
    println!("report: {report_path}");
    println!("playbooks: {playbooks_path}");
    println!(
        "status: {}",
        active
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-")
    );
    println!(
        "label: {}",
        active
            .get("label")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-")
    );
    println!();
    if let Some(description) = active
        .get("description")
        .and_then(serde_json::Value::as_str)
    {
        println!("{description}");
        println!();
    }
    println!("commands:");
    if let Some(commands) = active.get("commands").and_then(serde_json::Value::as_array) {
        for command in commands {
            if let Some(command) = command.as_str() {
                println!("  {command}");
            }
        }
    }
    Ok(())
}

fn run_scenarios(
    report_path: &str,
    universe_path: &str,
    scenarios_path: &str,
) -> anyhow::Result<()> {
    let report = read_json_report(report_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read report {report_path}"))?;
    let universe = read_json_report(universe_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read universe {universe_path}"))?;
    let scenarios = read_json_report(scenarios_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read scenarios {scenarios_path}"))?;
    let categories = category_map_from_universe(&universe);
    let positions = report
        .get("positions")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let nav = report
        .get("nav_usd")
        .and_then(serde_json::Value::as_str)
        .and_then(decimal_from_str)
        .unwrap_or(Decimal::ZERO);

    println!("# Scenario Stress");
    println!();
    println!("report: {report_path}");
    println!("scenarios: {scenarios_path}");
    println!("nav_usd: {:.2}", nav);
    println!();
    println!("| Scenario | Return % | PnL USD | Largest Loss |");
    println!("|:---------|---------:|--------:|:-------------|");

    if let Some(items) = scenarios.as_array() {
        for scenario in items {
            let id = scenario
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            let shocks = scenario_shock_map(
                scenario
                    .get("category_shocks_pct")
                    .unwrap_or(&serde_json::Value::Object(Default::default())),
            );
            let mut pnl = Decimal::ZERO;
            let mut largest_symbol = String::from("-");
            let mut largest_loss = Decimal::ZERO;
            for position in &positions {
                let symbol = position
                    .get("symbol")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("UNKNOWN");
                let value = json_decimal_field(position, "value_usd");
                let category = categories
                    .get(symbol)
                    .map(String::as_str)
                    .unwrap_or("uncategorized");
                let shock = shocks
                    .get(category)
                    .copied()
                    .or_else(|| shocks.get("uncategorized").copied())
                    .unwrap_or(Decimal::ZERO);
                let position_pnl = (value * shock / Decimal::from(100)).round_dp(2);
                pnl += position_pnl;
                if position_pnl < largest_loss {
                    largest_loss = position_pnl;
                    largest_symbol = symbol.to_string();
                }
            }
            let ret = if nav > Decimal::ZERO {
                (pnl / nav * Decimal::from(100)).round_dp(2)
            } else {
                Decimal::ZERO
            };
            println!(
                "| {:<20} | {:>8.2} | {:>7.2} | {} {:.2} |",
                id, ret, pnl, largest_symbol, largest_loss
            );
        }
    }
    Ok(())
}

fn run_watchlist(limit: usize) -> anyhow::Result<()> {
    let limit = limit.clamp(1, 50);
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let runtime = tokio::runtime::Runtime::new()?;
    let mut rows = runtime.block_on(async {
        let source = cmc_client::MockCmcClient::new();
        let snapshot = market_data::SnapshotBuilder::new(&source, &universe)
            .build()
            .await?;
        let mut rows = Vec::new();
        for asset in snapshot.assets {
            if asset.asset.category.is_stable() {
                continue;
            }
            let ret_24h = asset.ret_24h.map(to_f64).unwrap_or(0.0);
            let volatility = asset.volatility_1h.map(to_f64).unwrap_or(0.0);
            let liquidity = asset.liquidity_usd.map(to_f64).unwrap_or(0.0);
            let safety_penalty = ((100_i64 - asset.safety_score as i64).max(0) as f64) / 10.0;
            let liquidity_penalty = if liquidity > 0.0 && liquidity < 500_000.0 {
                8.0
            } else {
                0.0
            };
            let flag_penalty = asset.security_flags.len() as f64 * 6.0;
            let score = ret_24h.abs()
                + (volatility * 2.0)
                + safety_penalty
                + liquidity_penalty
                + flag_penalty;
            let status = if score >= 25.0 {
                "critical"
            } else if score >= 12.0 {
                "watch"
            } else {
                "normal"
            };
            rows.push((
                asset.asset.symbol,
                format!("{:?}", asset.asset.category).to_ascii_lowercase(),
                status.to_string(),
                score,
                ret_24h,
                volatility,
                liquidity,
                asset.safety_score,
            ));
        }
        anyhow::Ok(rows)
    })?;
    rows.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    rows.truncate(limit);

    println!("# Watchlist");
    println!();
    println!("limit: {limit}");
    println!("| Symbol | Category | Status | Score | 24h % | Vol % | Liquidity | Safety |");
    println!("|:-------|:---------|:-------|------:|------:|------:|----------:|-------:|");
    for (symbol, category, status, score, ret_24h, volatility, liquidity, safety) in rows {
        println!(
            "| {:<6} | {:<14} | {:<8} | {:>5.2} | {:>5.2} | {:>5.2} | {:>9.0} | {:>6} |",
            symbol, category, status, score, ret_24h, volatility, liquidity, safety
        );
    }
    Ok(())
}

fn run_liquidity(policy_path: &str, limit: usize) -> anyhow::Result<()> {
    let policy = read_json_report(policy_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read liquidity policy {policy_path}"))?;
    let universe_path = policy
        .get("universe_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(DEFAULT_UNIVERSE);
    let max_usage = json_decimal_or(&policy, "max_pool_usage_pct", Decimal::new(5, 1));
    let warning_usage = json_decimal_or(&policy, "warning_pool_usage_pct", Decimal::new(35, 2));
    let min_liquidity = json_decimal_or(&policy, "min_liquidity_usd", Decimal::from(500_000));
    let notional = json_decimal_or(&policy, "default_order_notional_usd", Decimal::from(1000));
    let universe = market_data::Universe::load(universe_path)?;
    let runtime = tokio::runtime::Runtime::new()?;
    let mut rows = runtime.block_on(async {
        let source = cmc_client::MockCmcClient::new();
        let snapshot = market_data::SnapshotBuilder::new(&source, &universe)
            .build()
            .await?;
        let mut rows = Vec::new();
        for asset in snapshot.assets {
            if asset.asset.category.is_stable() {
                continue;
            }
            let liquidity = asset.liquidity_usd.unwrap_or(Decimal::ZERO);
            let capacity = liquidity * max_usage / Decimal::from(100);
            let usage = if liquidity > Decimal::ZERO {
                notional / liquidity * Decimal::from(100)
            } else {
                Decimal::ZERO
            };
            let headroom = (capacity - notional).max(Decimal::ZERO);
            let status = if liquidity < min_liquidity || usage > max_usage {
                "blocking"
            } else if usage >= warning_usage {
                "watch"
            } else {
                "ok"
            };
            rows.push((
                asset.asset.symbol,
                status.to_string(),
                liquidity,
                capacity,
                usage,
                headroom,
            ));
        }
        anyhow::Ok(rows)
    })?;
    rows.sort_by(|a, b| a.5.cmp(&b.5));
    rows.truncate(limit.clamp(1, 50));

    println!("# Liquidity");
    println!();
    println!("policy: {policy_path}");
    println!("notional_usd: {:.2}", notional);
    println!("max_pool_usage_pct: {:.4}", max_usage);
    println!();
    println!("| Symbol | Status | Liquidity | Capacity | Usage % | Headroom |");
    println!("|:-------|:-------|----------:|---------:|--------:|---------:|");
    for (symbol, status, liquidity, capacity, usage, headroom) in rows {
        println!(
            "| {:<6} | {:<8} | {:>9.2} | {:>8.2} | {:>7.4} | {:>8.2} |",
            symbol, status, liquidity, capacity, usage, headroom
        );
    }
    Ok(())
}

fn run_costs(config_path: &str, amount_override: Option<&str>) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read cost config {config_path}"))?;
    let native_price = json_decimal_or(&config, "native_price_usd", Decimal::from(610));
    let gas_price_gwei = json_decimal_or(&config, "gas_price_gwei", Decimal::from(3));
    let quote_gas = json_decimal_or(&config, "quote_gas_units", Decimal::from(45_000));
    let swap_gas = json_decimal_or(&config, "swap_gas_units", Decimal::from(210_000));
    let approval_gas = json_decimal_or(&config, "approval_gas_units", Decimal::from(65_000));
    let approval_required = config
        .get("approval_required")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let amount = amount_override
        .and_then(decimal_from_str)
        .unwrap_or_else(|| {
            json_decimal_or(&config, "default_order_notional_usd", Decimal::from(1000))
        });
    let gas_units = quote_gas
        + swap_gas
        + if approval_required {
            approval_gas
        } else {
            Decimal::ZERO
        };
    let gas_usd = gas_cost_usd(gas_units, gas_price_gwei, native_price);
    let pool = Decimal::from(3_000_000);
    let slippage_pct = backtester::slippage::estimate_pct(amount, pool);
    let slippage_usd = (amount * slippage_pct / Decimal::from(100)).round_dp(4);
    let route_count = config
        .get("routes")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);

    println!("# Execution Costs");
    println!();
    println!("config: {config_path}");
    println!(
        "chain: {}",
        config
            .get("chain")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("bsc")
    );
    println!("amount_usd: {:.2}", amount);
    println!(
        "gas_price_gwei: {:.2} · native_price_usd: {:.2}",
        gas_price_gwei, native_price
    );
    println!();
    println!("| Route | Gas USD | Slippage USD | All-In USD | Cost BPS |");
    println!("|:------|--------:|-------------:|-----------:|---------:|");
    if let Some(routes) = config.get("routes").and_then(serde_json::Value::as_array) {
        for route in routes {
            let from = route
                .get("from")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("USDT");
            let to = route
                .get("to")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("WBNB");
            let all_in = gas_usd + slippage_usd;
            println!(
                "| {}->{} | {:>7.4} | {:>12.4} | {:>10.4} | {:>8.2} |",
                from,
                to,
                gas_usd,
                slippage_usd,
                all_in,
                cost_bps(all_in, amount)
            );
        }
    }
    println!();
    println!(
        "routes: {route_count} · total_gas_usd: {:.4} · total_slippage_usd: {:.4}",
        gas_usd * Decimal::from(route_count as i64),
        slippage_usd * Decimal::from(route_count as i64)
    );
    Ok(())
}

fn run_briefing(report_path: &str, config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read briefing config {config_path}"))?;
    let report = read_json_report(report_path);
    let title = config
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Submission Briefing");
    let kill_switch = report
        .as_ref()
        .and_then(|value| value.get("kill_switch"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let status = if report.is_none() || kill_switch {
        "blocking"
    } else {
        "ready"
    };

    println!("# {title}");
    println!();
    println!("status: {status}");
    println!("report: {report_path}");
    if let Some(report) = &report {
        println!(
            "run_id: {}",
            report
                .get("run_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-")
        );
        println!(
            "nav_usd: {}",
            report
                .get("nav_usd")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-")
        );
        println!(
            "wallet: {}",
            report
                .get("wallet_address")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-")
        );
    }
    println!();
    println!("claims:");
    if let Some(claims) = config.get("claims").and_then(serde_json::Value::as_array) {
        for claim in claims {
            if let Some(claim) = claim.as_str() {
                println!("  - {claim}");
            }
        }
    }
    println!();
    println!("artifacts:");
    if let Some(paths) = config
        .get("artifact_paths")
        .and_then(serde_json::Value::as_array)
    {
        for path in paths {
            if let Some(path) = path.as_str() {
                println!("  {path}");
            }
        }
    }
    println!();
    println!("demo commands:");
    if let Some(commands) = config
        .get("demo_commands")
        .and_then(serde_json::Value::as_array)
    {
        for command in commands {
            if let Some(command) = command.as_str() {
                println!("  {command}");
            }
        }
    }
    Ok(())
}

fn run_budget(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read budget config {config_path}"))?;
    let cost_path = config
        .get("cost_config_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("configs/costs/bsc_execution_costs.json");
    let report_path = config
        .get("report_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("data/run_report.json");
    let costs = read_json_report(cost_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read cost config {cost_path}"))?;
    let report = read_json_report(report_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read report {report_path}"))?;

    let nav = report
        .get("nav_usd")
        .and_then(serde_json::Value::as_str)
        .and_then(decimal_from_str)
        .unwrap_or(Decimal::ZERO);
    let daily_trades = json_decimal_or(&config, "daily_trade_target", Decimal::ONE);
    let planned_days = json_decimal_or(&config, "planned_competition_days", Decimal::from(7));
    let gas_float = json_decimal_or(&config, "operator_gas_float_usd", Decimal::ZERO);
    let max_daily = json_decimal_or(&config, "max_daily_execution_cost_usd", Decimal::from(5));
    let max_bps = json_decimal_or(&config, "max_cost_bps_per_trade", Decimal::from(25));
    let amount = json_decimal_or(&costs, "default_order_notional_usd", Decimal::from(1000));
    let gas_units = json_decimal_or(&costs, "quote_gas_units", Decimal::from(45_000))
        + json_decimal_or(&costs, "swap_gas_units", Decimal::from(210_000))
        + if costs
            .get("approval_required")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true)
        {
            json_decimal_or(&costs, "approval_gas_units", Decimal::from(65_000))
        } else {
            Decimal::ZERO
        };
    let gas_usd = gas_cost_usd(
        gas_units,
        json_decimal_or(&costs, "gas_price_gwei", Decimal::from(3)),
        json_decimal_or(&costs, "native_price_usd", Decimal::from(610)),
    );
    let slippage_pct = backtester::slippage::estimate_pct(amount, Decimal::from(3_000_000));
    let slippage_usd = amount * slippage_pct / Decimal::from(100);
    let per_trade = gas_usd + slippage_usd;
    let daily_cost = per_trade * daily_trades;
    let planned_cost = daily_cost * planned_days;
    let bps = cost_bps(per_trade, amount);
    let runway = if daily_cost > Decimal::ZERO {
        gas_float / daily_cost
    } else {
        Decimal::ZERO
    };
    let status = if daily_cost > max_daily || bps > max_bps {
        "blocking"
    } else if runway < planned_days {
        "watch"
    } else {
        "funded"
    };

    println!("# Trading Budget");
    println!();
    println!("status: {status}");
    println!("config: {config_path}");
    println!("report: {report_path}");
    println!("nav_usd: {:.2}", nav);
    println!("daily_trade_target: {:.2}", daily_trades);
    println!("runway_days: {:.2}", runway);
    println!("daily_cost_usd: {:.4} / max {:.2}", daily_cost, max_daily);
    println!("cost_bps: {:.2} / max {:.2}", bps, max_bps);
    println!("planned_cost_usd: {:.4}", planned_cost);
    Ok(())
}

fn run_mandates(config_path: &str) -> anyhow::Result<()> {
    let mandates = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read mandates config {config_path}"))?;
    println!("# Mandates");
    println!();
    println!("config: {config_path}");
    println!();
    println!("| ID | Hash | Max DD % | Position % | Reserve % | Slippage % |");
    println!("|:---|:-----|---------:|-----------:|----------:|-----------:|");
    if let Some(items) = mandates.as_array() {
        for item in items {
            let id = item
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            let mandate = item
                .get("mandate")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let compiled = policy_compiler::compile_mandate(mandate)?;
            println!(
                "| {} | {} | {:>8} | {:>10} | {:>9} | {:>10} |",
                id,
                compiled.hash,
                compiled.policy.max_total_drawdown_pct,
                compiled.policy.max_position_pct,
                compiled.policy.min_stable_reserve_pct,
                compiled.policy.max_slippage_pct
            );
        }
    }
    Ok(())
}

fn run_drift(policy_path: &str) -> anyhow::Result<()> {
    let policy = read_json_report(policy_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read drift policy {policy_path}"))?;
    let report_path = policy
        .get("report_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("data/run_report.json");
    let config_path = policy
        .get("config_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("configs/paper.toml");
    let warning = json_decimal_or(&policy, "warning_delta_pct", Decimal::from(3));
    let critical = json_decimal_or(&policy, "critical_delta_pct", Decimal::from(8));
    let max_turnover = json_decimal_or(&policy, "max_turnover_pct", Decimal::from(35));
    let stable_symbol = policy
        .get("stable_symbol")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("USDT");
    let report = read_json_report(report_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read report {report_path}"))?;
    let settings = Settings::load(config_path)?;
    let risk_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let risk_policy = risk_engine::RiskPolicy::from_json_str(&risk_raw)?;
    let cap = (to_f64(risk_policy.max_position_pct) - 1.0).max(1.0);
    let cfg = strategy_config(&settings, cap);
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let assets = universe.enabled_assets();
    let snapshot = build_warmed_snapshot(&assets, SNAPSHOT_WARMUP_STEPS);
    let nav = report
        .get("nav_usd")
        .and_then(serde_json::Value::as_str)
        .and_then(decimal_from_str)
        .unwrap_or_else(|| Decimal::from(10_000));
    let current = allocation_from_report(&report);
    let decision = StrategyEngine::new(cfg).decide(&snapshot, &current, nav);

    let mut rows = std::collections::BTreeMap::<String, (Decimal, Decimal)>::new();
    for (symbol, current_weight) in &current.weights_pct {
        rows.insert(symbol.clone(), (*current_weight, Decimal::ZERO));
    }
    for target in &decision.target_positions {
        rows.entry(target.symbol.clone())
            .and_modify(|entry| entry.1 = target.weight_pct)
            .or_insert((Decimal::ZERO, target.weight_pct));
    }

    let mut max_delta = Decimal::ZERO;
    let mut turnover = Decimal::ZERO;
    let mut ordered = Vec::new();
    for (symbol, (current_weight, target_weight)) in rows {
        let delta = target_weight - current_weight;
        let abs_delta = delta.abs();
        max_delta = max_delta.max(abs_delta);
        if symbol != stable_symbol {
            turnover += abs_delta;
        }
        ordered.push((symbol, current_weight, target_weight, delta, abs_delta));
    }
    ordered.sort_by(|a, b| b.4.cmp(&a.4));
    let status = if max_delta >= critical || turnover > max_turnover {
        "critical"
    } else if max_delta >= warning {
        "watch"
    } else {
        "aligned"
    };

    println!("# Portfolio Drift");
    println!();
    println!("status: {status}");
    println!("policy: {policy_path}");
    println!("report: {report_path}");
    println!("regime: {}", decision.regime.as_str());
    println!("max_delta_pct: {:.2}", max_delta);
    println!("turnover_pct: {:.2}", turnover);
    println!("turnover_usd: {:.2}", nav * turnover / Decimal::from(100));
    println!();
    println!("| Symbol | Current % | Target % | Delta % | Status |");
    println!("|:-------|----------:|---------:|--------:|:-------|");
    for (symbol, current_weight, target_weight, delta, abs_delta) in ordered {
        let row_status = if abs_delta >= critical {
            "critical"
        } else if abs_delta >= warning {
            "watch"
        } else {
            "normal"
        };
        println!(
            "| {:<6} | {:>9.2} | {:>8.2} | {:>7.2} | {} |",
            symbol, current_weight, target_weight, delta, row_status
        );
    }
    Ok(())
}

fn run_prizes(config_path: &str, report_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read prize map {config_path}"))?;
    let report = read_json_report(report_path);
    let report_present = report.is_some();
    let wallet_present = report
        .as_ref()
        .and_then(|value| value.get("wallet_address"))
        .and_then(serde_json::Value::as_str)
        .map(|value| !value.is_empty())
        .unwrap_or(false);
    let policy_hash_present = report
        .as_ref()
        .and_then(|value| value.get("policy_hash"))
        .and_then(serde_json::Value::as_str)
        .map(|value| !value.is_empty())
        .unwrap_or(false);
    let confirmed_txs = report
        .as_ref()
        .and_then(|value| value.get("trades"))
        .and_then(serde_json::Value::as_array)
        .map(|trades| !trades.is_empty())
        .unwrap_or(false);
    let daily_trade = report
        .as_ref()
        .and_then(|value| value.get("daily_trade"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(confirmed_txs);

    println!("# Prize Map");
    println!();
    println!("config: {config_path}");
    println!("report: {report_path}");
    println!();
    println!("| Category | Status | Facts | Evidence |");
    println!("|:---------|:-------|:------|:---------|");
    if let Some(items) = config.as_array() {
        for item in items {
            let label = item
                .get("label")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Prize");
            let required = item
                .get("required_facts")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            let passed = required
                .iter()
                .filter(|fact| {
                    let Some(name) = fact.as_str() else {
                        return false;
                    };
                    match name {
                        "report_present" => report_present,
                        "wallet_present" => wallet_present,
                        "policy_hash_present" => policy_hash_present,
                        "confirmed_txs" => confirmed_txs,
                        "daily_trade" => daily_trade,
                        _ => false,
                    }
                })
                .count();
            let status = if passed == required.len() {
                "ready"
            } else {
                "partial"
            };
            let evidence = item
                .get("evidence_paths")
                .and_then(serde_json::Value::as_array)
                .map(|paths| {
                    paths
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .take(3)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            println!(
                "| {} | {} | {}/{} | {} |",
                label,
                status,
                passed,
                required.len(),
                evidence
            );
        }
    }
    Ok(())
}

fn run_wallet_controls(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read wallet controls {config_path}"))?;
    let report_path = config
        .get("report_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("data/run_report.json");
    let report = read_json_report(report_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read report {report_path}"))?;
    let wallet = report
        .get("wallet_address")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let wallet_valid = common::Address::new(wallet).looks_valid();
    let max_allowance = json_decimal_or(&config, "max_allowance_usd", Decimal::from(1500));

    println!("# Wallet Controls");
    println!();
    println!("config: {config_path}");
    println!("report: {report_path}");
    println!("wallet: {wallet}");
    println!("wallet_valid: {wallet_valid}");
    println!(
        "approval_mode: {}",
        config
            .get("approval_mode")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("exact_or_low_cap")
    );
    println!("max_allowance_usd: {:.2}", max_allowance);
    println!();
    println!("| Spender | Status | Allowance USD | Address |");
    println!("|:--------|:-------|--------------:|:--------|");
    if let Some(spenders) = config.get("spenders").and_then(serde_json::Value::as_array) {
        for spender in spenders {
            let name = spender
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("spender");
            let address = spender
                .get("address")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let allowance = spender
                .get("allowance_usd")
                .and_then(|value| {
                    value
                        .as_f64()
                        .and_then(Decimal::from_f64)
                        .or_else(|| value.as_i64().map(Decimal::from))
                        .or_else(|| value.as_u64().map(Decimal::from))
                        .or_else(|| value.as_str().and_then(decimal_from_str))
                })
                .unwrap_or(Decimal::ZERO);
            let status =
                if !common::Address::new(address).looks_valid() || allowance > max_allowance {
                    "violation"
                } else if allowance == Decimal::ZERO {
                    "inactive"
                } else {
                    "ok"
                };
            println!("| {name} | {status} | {:.2} | {address} |", allowance);
        }
    }
    Ok(())
}

fn run_exit_triggers(policy_path: &str) -> anyhow::Result<()> {
    let policy = read_json_report(policy_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read exit policy {policy_path}"))?;
    let report_path = policy
        .get("report_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("data/run_report.json");
    let universe_path = policy
        .get("universe_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(DEFAULT_UNIVERSE);
    let report = read_json_report(report_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read report {report_path}"))?;
    let universe = market_data::Universe::load(universe_path)?;
    let assets = universe.enabled_assets();
    let snapshot = build_warmed_snapshot(&assets, SNAPSHOT_WARMUP_STEPS);

    let stop_loss = json_decimal_or(&policy, "stop_loss_pct", Decimal::from(12));
    let take_profit = json_decimal_or(&policy, "take_profit_pct", Decimal::from(25));
    let warning_loss = json_decimal_or(&policy, "warning_loss_pct", Decimal::from(8));
    let warning_gain = json_decimal_or(&policy, "warning_gain_pct", Decimal::from(18));
    let ret_exit = json_decimal_or(&policy, "ret_24h_exit_pct", Decimal::from(-12));

    let mut rows = Vec::new();
    let mut exit_count = 0usize;
    let mut watch_count = 0usize;
    for position in report
        .get("positions")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let symbol = position
            .get("symbol")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string();
        let weight = decimal_value(position.get("weight_pct")).unwrap_or(Decimal::ZERO);
        let value = decimal_value(position.get("value_usd")).unwrap_or(Decimal::ZERO);
        let ret_24h = snapshot
            .get(&symbol)
            .and_then(|asset| asset.ret_24h)
            .unwrap_or(Decimal::ZERO);
        let status = if ret_24h <= -stop_loss || ret_24h <= ret_exit {
            exit_count += 1;
            "exit"
        } else if ret_24h >= take_profit {
            exit_count += 1;
            "take_profit"
        } else if ret_24h <= -warning_loss || ret_24h >= warning_gain {
            watch_count += 1;
            "watch"
        } else {
            "hold"
        };
        rows.push((symbol, status, weight, value, ret_24h));
    }
    rows.sort_by(|a, b| a.4.cmp(&b.4));

    println!("# Exit Triggers");
    println!();
    println!("policy: {policy_path}");
    println!("report: {report_path}");
    println!("universe: {universe_path}");
    println!(
        "status: {}",
        if exit_count > 0 {
            "exit"
        } else if watch_count > 0 {
            "watch"
        } else {
            "hold"
        }
    );
    println!("thresholds: stop_loss={}%, take_profit={}%, warning_loss={}%, warning_gain={}%, ret_24h_exit={}%", stop_loss, take_profit, warning_loss, warning_gain, ret_exit);
    println!();
    println!("| Symbol | Status | Weight % | Value USD | 24h % |");
    println!("|:-------|:-------|---------:|----------:|------:|");
    for (symbol, status, weight, value, ret_24h) in rows {
        println!(
            "| {:<6} | {:<11} | {:>8.2} | {:>9.2} | {:>5.2} |",
            symbol, status, weight, value, ret_24h
        );
    }
    Ok(())
}

fn run_audit_manifest(config_path: &str) -> anyhow::Result<()> {
    let manifest = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read audit manifest {config_path}"))?;
    let artifacts = manifest
        .get("artifacts")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut present = 0usize;
    let mut missing_required = 0usize;
    let mut total_bytes = 0u64;
    let mut rows = Vec::new();
    for artifact in artifacts {
        let label = artifact
            .get("label")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("artifact")
            .to_string();
        let path = artifact
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string();
        let required = artifact
            .get("required")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let bytes = std::fs::read(&path).ok();
        let exists = bytes.is_some();
        if exists {
            present += 1;
        } else if required {
            missing_required += 1;
        }
        let size = bytes.as_ref().map(|b| b.len() as u64).unwrap_or(0);
        total_bytes += size;
        let hash = bytes
            .as_ref()
            .map(|b| policy_compiler::policy_hash(b))
            .unwrap_or_else(|| "-".to_string());
        rows.push((label, path, required, exists, size, hash));
    }

    println!("# Audit Manifest");
    println!();
    println!("config: {config_path}");
    println!(
        "name: {}",
        manifest
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Submission Audit")
    );
    println!(
        "status: {}",
        if missing_required == 0 {
            "ready"
        } else {
            "missing_required"
        }
    );
    println!("artifacts: {present}/{}", rows.len());
    println!("missing_required: {missing_required}");
    println!("total_bytes: {total_bytes}");
    println!();
    println!("| Artifact | Status | Bytes | SHA-256 |");
    println!("|:---------|:-------|------:|:--------|");
    for (label, path, required, exists, size, hash) in rows {
        let status = if exists {
            "present"
        } else if required {
            "missing"
        } else {
            "optional"
        };
        let short_hash = if hash.len() > 20 {
            format!("{}...{}", &hash[..12], &hash[hash.len() - 8..])
        } else {
            hash
        };
        println!("| {label} ({path}) | {status} | {size} | {short_hash} |");
    }

    println!();
    println!("routes:");
    if let Some(routes) = manifest.get("routes").and_then(serde_json::Value::as_array) {
        for route in routes {
            if let Some(path) = route.as_str() {
                println!("  {path}");
            }
        }
    }
    Ok(())
}

fn run_bnb_sdk(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read BNB SDK map {config_path}"))?;
    println!("# BNB Agent SDK Map");
    println!();
    println!("config: {config_path}");
    println!(
        "source_repo: {}",
        config
            .get("source_repo")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-")
    );
    println!(
        "local_clone: {}",
        config
            .get("local_clone")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-")
    );
    println!(
        "network: {} chain_id={}",
        config
            .get("network")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("bsc-mainnet"),
        config
            .get("chain_id")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(56)
    );
    println!(
        "competition_contract: {}",
        config
            .get("competition_contract")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(COMPETITION_CONTRACT)
    );
    println!(
        "bsctrace: {}",
        config
            .get("competition_contract_bsctrace")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("https://bsctrace.com/address/0x212c61b9b72c95d95bf29cf032f5e5635629aed5")
    );
    println!();
    println!("| SDK Module | Status | Guardrail Surface |");
    println!("|:-----------|:-------|:------------------|");
    if let Some(modules) = config
        .get("sdk_modules")
        .and_then(serde_json::Value::as_array)
    {
        for module in modules {
            let name = module
                .get("module")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("module");
            let status = module
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("mapped");
            let surface = module
                .get("guardrail_surface")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-");
            println!("| {name} | {status} | {surface} |");
        }
    }
    println!();
    println!("contracts:");
    if let Some(contracts) = config
        .get("sdk_contracts")
        .and_then(serde_json::Value::as_object)
    {
        for (name, address) in contracts {
            println!("  {name}: {}", address.as_str().unwrap_or("-"));
        }
    }
    Ok(())
}

fn run_commerce(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read commerce config {config_path}"))?;
    let report_path = config
        .get("report_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("data/run_report.json");
    let report = read_json_report(report_path).unwrap_or_else(|| serde_json::json!({}));
    let wallet = report
        .get("wallet_address")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("-");
    let policy_hash = report
        .get("policy_hash")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("-");
    let report_hash = report
        .get("report_hash")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("-");

    println!("# ERC-8183 Commerce");
    println!();
    println!("config: {config_path}");
    println!(
        "network: {} chain_id={}",
        config
            .get("network")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("bsc-mainnet"),
        config
            .get("chain_id")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(56)
    );
    println!(
        "service_price: {:.2} {}",
        json_f64(&config, "service_price_usd").unwrap_or(0.0),
        config
            .get("payment_token_symbol")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("US")
    );
    println!("wallet: {wallet}");
    println!("policy_hash: {policy_hash}");
    println!("report_hash: {report_hash}");
    println!();
    println!("contracts:");
    for key in [
        "payment_token",
        "commerce_proxy",
        "router_proxy",
        "policy",
        "erc8004_registry",
    ] {
        println!(
            "  {key}: {}",
            config
                .get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-")
        );
    }
    println!();
    println!("| State | Guardrail Surface | Description |");
    println!("|:------|:------------------|:------------|");
    if let Some(steps) = config
        .get("job_lifecycle")
        .and_then(serde_json::Value::as_array)
    {
        for step in steps {
            let state = step
                .get("state")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-");
            let surface = step
                .get("guardrail_surface")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-");
            let description = step
                .get("description")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-");
            println!("| {state} | {surface} | {description} |");
        }
    }
    println!();
    println!("deliverables:");
    if let Some(deliverables) = config
        .get("deliverables")
        .and_then(serde_json::Value::as_array)
    {
        for deliverable in deliverables {
            if let Some(path) = deliverable.as_str() {
                println!("  {path}");
            }
        }
    }
    Ok(())
}

fn run_signing_policy(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read signing policy {config_path}"))?;
    let payer_env = config
        .get("payer_wallet_env")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("CMC_X402_FROM");
    let payer = std::env::var(payer_env).unwrap_or_else(|_| {
        config
            .get("fallback_payer_wallet")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(DEFAULT_AGENT_WALLET)
            .to_string()
    });
    let resources = config
        .get("resources")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let first = resources
        .first()
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let requirements = cmc_client::x402::PaymentRequirements {
        scheme: first
            .get("scheme")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("exact")
            .to_string(),
        network: first
            .get("network")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("bsc")
            .to_string(),
        max_amount_required: first
            .get("amount_base_units")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("100000")
            .to_string(),
        asset: config
            .get("payment_token")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
        pay_to: first
            .get("pay_to")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
        resource: first
            .get("resource")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
    };
    let unsigned = cmc_client::x402::PaymentPayload::from_requirements(&requirements, &payer);
    let authorization = unsigned.authorization_json();
    let signed = twak_client::x402::sign_authorization(&authorization, &payer);

    println!("# x402 Signing Policy");
    println!();
    println!("config: {config_path}");
    println!(
        "mode: {}",
        config
            .get("mode")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("strict_default")
    );
    println!(
        "payment_token: {}",
        config
            .get("payment_token")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-")
    );
    println!(
        "max_per_call_base_units: {}",
        config
            .get("max_per_call_base_units")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("0")
    );
    println!(
        "session_budget_base_units: {}",
        config
            .get("session_budget_base_units")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("0")
    );
    println!("sample_payer: {payer}");
    println!(
        "sample_authorization_hash: {}",
        policy_compiler::policy_hash(authorization.as_bytes())
    );
    println!("sample_signature: {}", signed.signature);
    println!();
    println!("allowlist:");
    if let Some(values) = config
        .get("primary_type_allowlist")
        .and_then(serde_json::Value::as_array)
    {
        for value in values {
            if let Some(value) = value.as_str() {
                println!("  {value}");
            }
        }
    }
    println!("denylist:");
    if let Some(values) = config
        .get("primary_type_denylist")
        .and_then(serde_json::Value::as_array)
    {
        for value in values {
            if let Some(value) = value.as_str() {
                println!("  {value}");
            }
        }
    }
    println!();
    println!("| Resource | Amount | Network | Pay To |");
    println!("|:---------|-------:|:--------|:-------|");
    for resource in resources {
        let label = resource
            .get("label")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("resource");
        let amount = resource
            .get("amount_base_units")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("0");
        let network = resource
            .get("network")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("bsc");
        let pay_to = resource
            .get("pay_to")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-");
        println!("| {label} | {amount} | {network} | {pay_to} |");
    }
    Ok(())
}

fn run_heartbeat(config_path: &str) -> anyhow::Result<()> {
    let config = read_json_report(config_path)
        .ok_or_else(|| anyhow::anyhow!("failed to read heartbeat config {config_path}"))?;
    let report_path = config
        .get("report_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("data/run_report.json");
    let report = read_json_report(report_path).unwrap_or_else(|| serde_json::json!({}));
    let nav = report
        .get("nav_usd")
        .and_then(serde_json::Value::as_str)
        .and_then(decimal_from_str)
        .unwrap_or_else(|| Decimal::from(10_000));
    let max_pct = json_decimal_or(&config, "max_heartbeat_trade_pct", Decimal::from(2));
    let min_notional = json_decimal_or(&config, "min_notional_usd", Decimal::from(25));
    let target = json_decimal_or(&config, "target_notional_usd", Decimal::from(100));
    let max_notional = json_decimal_or(&config, "max_notional_usd", Decimal::from(200));
    let planned = target
        .min(max_notional)
        .min(nav * max_pct / Decimal::from(100))
        .max(min_notional);
    let daily_trade = report
        .get("daily_trade")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let from = config
        .get("preferred_pair")
        .and_then(|pair| pair.get("from"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("USDT");
    let to = config
        .get("preferred_pair")
        .and_then(|pair| pair.get("to"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("WBNB");

    println!("# Daily Trade Heartbeat");
    println!();
    println!("config: {config_path}");
    println!("report: {report_path}");
    println!(
        "status: {}",
        if daily_trade {
            "satisfied"
        } else {
            "due_or_unverified"
        }
    );
    println!(
        "requirement: {} trade/day, max heartbeat {}% NAV",
        config
            .get("min_trades_per_day")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1),
        max_pct
    );
    println!("nav_usd: {:.2}", nav);
    println!("planned_pair: {from} -> {to}");
    println!("planned_notional_usd: {:.2}", planned);
    println!(
        "execution_path: {}",
        config
            .get("execution_path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("risk_gate -> TWAK quote -> final risk -> TWAK swap")
    );
    println!();
    println!(
        "quote_command: cargo run -p guardrail-cli -- quote --from {from} --to {to} --amount {:.2}",
        planned
    );
    Ok(())
}

/// Parse a TWAK transport string into a [`TwakTransport`].
///
/// Anything other than the four known surfaces falls back to the offline mock,
/// with a note, so paper mode keeps working on a typo.
fn parse_transport(transport: &str) -> twak_client::TwakTransport {
    match transport.to_ascii_lowercase().as_str() {
        "mock" => twak_client::TwakTransport::Mock,
        "rest" => twak_client::TwakTransport::Rest,
        "mcp" => twak_client::TwakTransport::Mcp,
        "cli" => twak_client::TwakTransport::Cli,
        other => {
            println!("note: unknown transport '{other}'; falling back to offline mock");
            twak_client::TwakTransport::Mock
        }
    }
}

/// Register the agent for the competition through TWAK (self-custody).
///
/// Builds a tokio runtime (keeping `main` synchronous), resolves the executor
/// for the chosen transport, fetches the wallet address, and submits the
/// registration. On any failure it prints a friendly message pointing at the
/// `twak compete register` manual fallback. The default `mock` transport keeps
/// this fully offline and deterministic.
fn run_register(transport: &str, base_url: Option<&str>, autonomous: bool) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    let twak_transport = parse_transport(transport);

    // Prefer an explicit flag, then the environment, before any default.
    let env_base_url = std::env::var("TWAK_BASE_URL").ok();
    let resolved_base_url = base_url.or(env_base_url.as_deref());

    let result: Result<(common::Address, twak_client::TxReceipt), twak_client::TwakError> = runtime
        .block_on(async {
            let executor =
                twak_client::executor_from(twak_transport, resolved_base_url, autonomous);
            let wallet = executor.wallet_address().await?;
            let receipt = executor.register_competition().await?;
            Ok((wallet, receipt))
        });

    match result {
        Ok((wallet, receipt)) => {
            println!("transport: {transport}");
            println!("wallet_address: {}", wallet.0);
            println!("competition_contract: {COMPETITION_CONTRACT}");
            println!("tx_hash: {}", receipt.tx_hash);
            println!("bscscan: https://bscscan.com/tx/{}", receipt.tx_hash);
        }
        Err(e) => {
            println!("registration via TWAK failed: {e}");
            println!("fallback: run `twak compete register` (self-custody) to register manually");
            println!("competition_contract: {COMPETITION_CONTRACT}");
        }
    }

    Ok(())
}

/// Default agent wallet used when `AGENT_WALLET` is not set in the environment.
const DEFAULT_AGENT_WALLET: &str = "0xA9e5C0FfEe0000000000000000000000000A1b2C3";

/// Print the agent's BNB identity and proof commitments as pretty JSON.
fn run_identity(config: &str) -> anyhow::Result<()> {
    let settings = Settings::load(config)?;
    let policy_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let policy_hash = bnb_agent::sha256_hex_str(&policy_raw);

    let wallet = std::env::var("AGENT_WALLET").unwrap_or_else(|_| DEFAULT_AGENT_WALLET.to_string());

    let identity = bnb_agent::AgentIdentity::new(settings.app.name.clone(), wallet.clone());
    let agent_id = identity.agent_id();

    let metadata = bnb_agent::AgentMetadata {
        name: settings.app.name.clone(),
        description: "Regime-routed, risk-guarded alpha agent on BSC".to_string(),
        strategy_hash: bnb_agent::sha256_hex_str("regime-routed-bsc-alpha"),
        policy_hash: policy_hash.clone(),
        version: "0.1.0".to_string(),
    };

    let erc8004 = bnb_agent::Erc8004Record::build(&identity, &metadata);
    let report_hash = bnb_agent::sha256_hex_str("pending");
    let proof = bnb_agent::AgentProof::new(
        agent_id.clone(),
        wallet.clone(),
        policy_hash.clone(),
        report_hash,
    );

    let output = serde_json::json!({
        "agent_id": agent_id,
        "wallet": wallet,
        "address_url": proof.address_url(),
        "policy_hash": policy_hash,
        "metadata": metadata,
        "erc8004": erc8004,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Default HTTP timeout (ms) for the live CMC REST client.
const CMC_TIMEOUT_MS: u64 = 10_000;

/// Print a live market table for the eligible universe.
///
/// With `--live` and `CMC_API_KEY` set, pulls from the real CMC REST API;
/// otherwise falls back to the deterministic mock (and says so).
fn run_markets(config: &str, live: bool) -> anyhow::Result<()> {
    let _settings = Settings::load(config)?;
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        // Pick a data source: live REST when requested and keyed, else mock.
        let source: Box<dyn cmc_client::CmcDataSource> = match (live, std::env::var("CMC_API_KEY"))
        {
            (true, Ok(api_key)) if !api_key.is_empty() => {
                Box::new(cmc_client::CmcRestClient::new(api_key, CMC_TIMEOUT_MS)?)
            }
            (true, _) => {
                println!("note: --live requested but CMC_API_KEY is not set; using mock data");
                Box::new(cmc_client::MockCmcClient::new())
            }
            (false, _) => Box::new(cmc_client::MockCmcClient::new()),
        };

        let snapshot = market_data::SnapshotBuilder::new(source.as_ref(), &universe)
            .build()
            .await?;

        match &snapshot.fear_greed {
            Some(fg) => println!("Fear & Greed: {} ({})", fg.value, fg.classification),
            None => println!("Fear & Greed: unavailable"),
        }
        println!();

        println!(
            "{:<8} | {:>14} | {:>8} | {:>16} | {:>16} | {:>6}",
            "SYMBOL", "PRICE", "24H%", "VOL_24H", "LIQUIDITY", "SAFETY"
        );
        println!("{}", "-".repeat(82));

        for a in &snapshot.assets {
            let ret_24h = a
                .ret_24h
                .map(|r| format!("{:>8.2}", to_f64(r)))
                .unwrap_or_else(|| format!("{:>8}", "n/a"));
            let liquidity = a
                .liquidity_usd
                .map(|l| format!("{:>16.2}", to_f64(l)))
                .unwrap_or_else(|| format!("{:>16}", "n/a"));

            println!(
                "{:<8} | {:>14.6} | {} | {:>16.2} | {} | {:>6}",
                a.asset.symbol,
                to_f64(a.price_usd),
                ret_24h,
                to_f64(a.volume_24h_usd),
                liquidity,
                a.safety_score,
            );
        }

        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

fn run_quote(from: &str, to: &str, amount: &str) -> anyhow::Result<()> {
    let amount = Decimal::from_str(amount)
        .map_err(|_| anyhow::anyhow!("amount must be a number, got '{amount}'"))?;
    // AMM-style quote against a nominal pool.
    let pool = Decimal::from(3_000_000);
    let slippage = backtester::slippage::estimate_pct(amount, pool);
    let expected_out = (amount * (Decimal::ONE - slippage / Decimal::from(100))).round_dp(2);

    let quote = serde_json::json!({
        "from": from,
        "to": to,
        "amount_in_usd": amount.to_string(),
        "expected_out_usd": expected_out.to_string(),
        "slippage_pct": slippage.to_string(),
        "venue": "twak/pancakeswap",
    });
    println!("{}", serde_json::to_string_pretty(&quote)?);
    Ok(())
}

/// Fear/greed level used when evolving the deterministic indicator price path.
const INDICATOR_FEAR_GREED: u32 = 60;

/// Build a deterministic close-price series and print latest indicator values.
///
/// The series is generated by evolving `backtester::synthetic::initial_price`
/// with `step_return_24h_pct`, so output is reproducible for a given symbol and
/// step count. Indicators are computed over the resulting closes.
fn run_indicators(symbol: &str, steps: u32) -> anyhow::Result<()> {
    if steps == 0 {
        anyhow::bail!("steps must be greater than 0");
    }

    // Evolve a deterministic price path into a series of closes.
    let mut price = backtester::synthetic::initial_price(symbol);
    let mut closes: Vec<f64> = Vec::with_capacity(steps as usize);
    for step in 0..steps {
        let r = backtester::synthetic::step_return_24h_pct(symbol, step, INDICATOR_FEAR_GREED);
        price *= Decimal::ONE + r / Decimal::from(100);
        closes.push(to_f64(price));
    }

    let latest_close = closes
        .last()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("no closes generated"))?;

    // Compute the classic indicator set over the closes.
    let ema_series = indicators::ema(&closes, 12);
    let sma_series = indicators::sma(&closes, 12);
    let rsi_series = indicators::rsi(&closes, 14);
    let macd_out = indicators::macd(&closes, 12, 26, 9);
    let bb = indicators::bollinger(&closes, 20, 2.0);

    /// Format the last value of a series, or `n/a` when empty/insufficient data.
    fn last_fmt(series: &[f64]) -> String {
        series
            .last()
            .map(|v| format!("{v:.6}"))
            .unwrap_or_else(|| "n/a".to_string())
    }

    println!("# Indicators · {symbol}");
    println!();
    println!("steps: {steps} · fear/greed: {INDICATOR_FEAR_GREED}");
    println!();
    println!("latest close : {latest_close:.6}");
    println!("EMA(12)      : {}", last_fmt(&ema_series));
    println!("SMA(12)      : {}", last_fmt(&sma_series));
    println!("RSI(14)      : {}", last_fmt(&rsi_series));
    println!(
        "MACD(12,26,9): line={} signal={} hist={}",
        last_fmt(&macd_out.macd),
        last_fmt(&macd_out.signal),
        last_fmt(&macd_out.histogram),
    );
    println!(
        "Bollinger(20,2.0): upper={} mid={} lower={}",
        last_fmt(&bb.upper),
        last_fmt(&bb.mid),
        last_fmt(&bb.lower),
    );

    Ok(())
}

/// Render an offline Markdown run report from the agent's persisted JSON state.
///
/// If the file is missing, prints a friendly note and returns `Ok(())` so the
/// command is safe to run before the agent has produced any state.
fn run_report(report_path: &str) -> anyhow::Result<()> {
    let raw = match std::fs::read_to_string(report_path) {
        Ok(raw) => raw,
        Err(_) => {
            println!(
                "note: run report '{report_path}' not found; run the agent first to generate it"
            );
            return Ok(());
        }
    };

    let state: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("failed to parse run report '{report_path}': {e}"))?;

    let run_id = json_str(&state, "run_id");
    let mode = json_str(&state, "mode");
    let regime = json_str(&state, "regime");

    println!("# Guardrail Run Report");
    println!();
    println!("- run_id: {run_id}");
    println!("- mode: {mode}");
    println!("- regime: {regime}");

    let starting_nav = json_f64(&state, "starting_nav_usd");
    let current_nav = json_f64(&state, "nav_usd");
    let total_return = match (starting_nav, current_nav) {
        (Some(start), Some(now)) if start != 0.0 => Some((now - start) / start * 100.0),
        _ => None,
    };

    let kill_switch = state
        .get("kill_switch")
        .and_then(|v| v.as_bool())
        .map(|b| if b { "TRIGGERED" } else { "ok" }.to_string())
        .unwrap_or_else(|| json_str(&state, "kill_switch"));

    let events = state
        .get("events")
        .and_then(|v| v.as_u64())
        .map(|n| n.to_string())
        .unwrap_or_else(|| json_str(&state, "events"));

    println!();
    println!("## Metrics");
    println!();
    println!("| Metric | Value |");
    println!("|:-------|------:|");
    println!(
        "| Starting NAV (USD) | {} |",
        starting_nav
            .map(|n| format!("{n:.2}"))
            .unwrap_or_else(|| "n/a".to_string())
    );
    println!(
        "| Current NAV (USD) | {} |",
        current_nav
            .map(|n| format!("{n:.2}"))
            .unwrap_or_else(|| "n/a".to_string())
    );
    println!(
        "| Total Return % | {} |",
        total_return
            .map(|n| format!("{n:.2}"))
            .unwrap_or_else(|| "n/a".to_string())
    );
    println!(
        "| Total Drawdown % | {} |",
        json_num_fmt(&state, "total_drawdown_pct")
    );
    println!("| Events | {events} |");
    println!("| Kill Switch | {kill_switch} |");

    println!();
    println!("## Positions");
    println!();
    println!("| Symbol | Weight % | Value (USD) |");
    println!("|:-------|---------:|------------:|");
    match state.get("positions").and_then(|v| v.as_array()) {
        Some(positions) if !positions.is_empty() => {
            for p in positions {
                let symbol = json_str(p, "symbol");
                let weight = json_num_fmt(p, "weight_pct");
                let value = json_num_fmt(p, "value_usd");
                println!("| {symbol} | {weight} | {value} |");
            }
        }
        _ => {
            println!("| _none_ | n/a | n/a |");
        }
    }

    let wallet = json_str(&state, "wallet_address");
    let policy_hash = json_str(&state, "policy_hash");

    println!();
    println!("## Commitments");
    println!();
    println!("- wallet_address: {wallet}");
    println!("- policy_hash: {policy_hash}");

    Ok(())
}

/// One-line pitch reused by the submission summary.
const SUBMISSION_PITCH: &str =
    "NL mandate -> hashed risk policy -> regime-routed alpha -> dual risk gate \
     + kill switch -> TWAK self-custody execution, fully logged and replayable.";

/// Track the agent competes in.
const SUBMISSION_TRACK: &str = "Track 1 — Autonomous Trading Agents";

/// Print a concise DoraHacks submission summary from the latest run report.
///
/// Reads the persisted `run_report.json` (run id, mode, NAV, drawdown, and any
/// agent-identity / proof fields) and prints the project pitch, track, the run's
/// proof fields, the competition contract, and a pointer to `SUBMISSION.md`. If
/// the report is missing it prints a friendly note and returns `Ok(())` so the
/// command is safe to run before the agent has produced any state.
fn run_submission(report_path: &str) -> anyhow::Result<()> {
    println!("# Guardrail Alpha — Submission Summary");
    println!();
    println!("pitch  : {SUBMISSION_PITCH}");
    println!("track  : {SUBMISSION_TRACK}");
    println!("prizes : Best Use of TWAK · Agent Hub (CMC) · BNB AI Agent SDK");
    println!();
    println!("competition_contract: {COMPETITION_CONTRACT}");
    println!("writeup             : SUBMISSION.md (repo root)");
    println!();

    let raw = match std::fs::read_to_string(report_path) {
        Ok(raw) => raw,
        Err(_) => {
            println!(
                "note: run report '{report_path}' not found; run the agent first \
                 (e.g. `cargo run -p guardrail-agent -- --config configs/paper.toml`)."
            );
            return Ok(());
        }
    };

    let state: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("failed to parse run report '{report_path}': {e}"))?;

    let run_id = json_str(&state, "run_id");
    let mode = json_str(&state, "mode");

    let nav = json_f64(&state, "nav_usd")
        .map(|n| format!("{n:.2}"))
        .unwrap_or_else(|| "n/a".to_string());
    let drawdown = json_num_fmt(&state, "total_drawdown_pct");

    let kill_switch = state
        .get("kill_switch")
        .and_then(|v| v.as_bool())
        .map(|b| if b { "TRIGGERED" } else { "ok" }.to_string())
        .unwrap_or_else(|| json_str(&state, "kill_switch"));

    println!("## Latest run");
    println!();
    println!("- run_id      : {run_id}");
    println!("- mode        : {mode}");
    println!("- NAV (USD)   : {nav}");
    println!("- drawdown %  : {drawdown}");
    println!("- kill switch : {kill_switch}");

    let wallet = json_str(&state, "wallet_address");
    let policy_hash = json_str(&state, "policy_hash");
    let report_hash = json_str(&state, "report_hash");

    println!();
    println!("## Proof");
    println!();
    println!("- agent wallet : {wallet}");
    println!("- policy_hash  : {policy_hash}");
    println!("- report_hash  : {report_hash}");
    println!("- contract     : {COMPETITION_CONTRACT}");
    println!();
    println!("See SUBMISSION.md for the full writeup, strategy, and how-to-run.");

    Ok(())
}
