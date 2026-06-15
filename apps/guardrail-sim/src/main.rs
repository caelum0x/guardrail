//! Guardrail Alpha scenario sweep.
//!
//! Runs the real backtest engine across a range of Fear & Greed inputs to show
//! how the regime-routed strategy adapts: how exposure, return, and drawdown
//! shift from fearful (defensive) to greedy (constructive) markets. Reuses the
//! production strategy + risk + portfolio path — only the sentiment input varies.
//!
//! With `--walk-forward`, instead runs a sequence of windows whose sentiment
//! ramps across regimes, then prints a per-window table plus an aggregate line.

use backtester::{run_backtest, walk_forward, BacktestConfig, WalkForwardConfig};
use clap::Parser;
use common::decimal::to_f64;
use market_data::Universe;
use risk_engine::RiskPolicy;
use rust_decimal::Decimal;
use strategy_engine::StrategyConfig;

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

/// Load and apply a named preset's overrides onto `cfg`, preserving
/// `max_position_weight_pct` (the policy cap). On a missing file/preset this
/// returns `cfg` unchanged so callers fall back to current behavior.
///
/// Returns the (possibly updated) config plus a human-readable note about which
/// preset is active, so it can be printed once by the caller.
fn apply_preset(mut cfg: StrategyConfig, preset: &str) -> (StrategyConfig, String) {
    let raw = match std::fs::read_to_string(STRATEGY_PRESETS_PATH) {
        Ok(raw) => raw,
        Err(_) => {
            return (
                cfg,
                format!(
                    "note: preset file '{STRATEGY_PRESETS_PATH}' not found; using default config"
                ),
            );
        }
    };
    let presets: std::collections::HashMap<String, PresetOverrides> =
        match serde_json::from_str(&raw) {
            Ok(presets) => presets,
            Err(e) => {
                return (
                    cfg,
                    format!(
                    "note: failed to parse '{STRATEGY_PRESETS_PATH}' ({e}); using default config"
                ),
                );
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
            (cfg, format!("active preset: {preset}"))
        }
        None => (
            cfg,
            format!("note: preset '{preset}' not found in '{STRATEGY_PRESETS_PATH}'; using default config"),
        ),
    }
}

/// Default sentiment ramp used for walk-forward when no `--fear-greed` list is
/// supplied: fearful -> neutral -> greedy -> cooling off.
const DEFAULT_WF_PATH: [u32; 6] = [25, 40, 55, 70, 85, 60];

#[derive(Debug, Parser)]
#[command(
    name = "guardrail-sim",
    about = "Sweep the backtest across sentiment regimes"
)]
struct Cli {
    /// Eligible-asset universe file.
    #[arg(long, default_value = DEFAULT_UNIVERSE)]
    universe: String,
    /// Risk policy JSON file.
    #[arg(long, default_value = "configs/risk_policy.paper.json")]
    policy: String,
    /// Steps per backtest (per window in walk-forward mode).
    #[arg(long, default_value_t = 60)]
    steps: u32,
    /// Comma-separated Fear & Greed values to sweep.
    #[arg(long, default_value = "20,35,50,65,80")]
    fear_greed: String,
    /// Run walk-forward analysis instead of the sentiment sweep.
    #[arg(long, default_value_t = false)]
    walk_forward: bool,
    /// Number of sequential windows for walk-forward mode.
    #[arg(long, default_value_t = 6)]
    windows: u32,
    /// Strategy preset to apply (see configs/strategy_presets.json).
    #[arg(long, default_value = DEFAULT_PRESET)]
    preset: String,
}

/// Build the strategy config shared by both modes: production entry/hold scores
/// with a position cap derived from the risk policy, then apply the selected
/// preset's overrides (keeping the policy-derived position cap).
fn strategy_config(cap: f64, preset: &str) -> StrategyConfig {
    let base = StrategyConfig {
        max_position_weight_pct: cap,
        min_score_to_enter: 0.55,
        min_score_to_hold: 0.45,
        ..StrategyConfig::default()
    };
    let (cfg, _note) = apply_preset(base, preset);
    cfg
}

/// Parse the comma-separated `--fear-greed` flag into a list of readings.
fn parse_fear_greed(raw: &str) -> Vec<u32> {
    raw.split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect()
}

/// Run the classic per-regime sentiment sweep.
fn run_sweep(
    universe: &Universe,
    policy: &RiskPolicy,
    cap: f64,
    steps: u32,
    fg_values: &[u32],
    preset: &str,
) {
    println!("Guardrail Alpha — sentiment sweep ({steps} steps each)\n");
    println!(
        "{:>10}  {:>12}  {:>12}  {:>12}  {:>12}  {:>7}",
        "fear/greed", "return %", "buy&hold %", "excess %", "max dd %", "trades"
    );
    println!("{}", "-".repeat(78));

    for &fg in fg_values {
        // Fresh strategy/policy per run; only sentiment changes.
        let run = run_backtest(
            universe,
            policy.clone(),
            strategy_config(cap, preset),
            BacktestConfig {
                steps,
                starting_usd: Decimal::from(10_000),
                fear_greed: fg,
            },
        );
        let m = &run.metrics;
        println!(
            "{:>10}  {:>12}  {:>12}  {:>12}  {:>12}  {:>7}",
            fg,
            m.total_return_pct,
            run.benchmark_return_pct,
            run.excess_return_pct,
            m.max_drawdown_pct,
            m.trade_count
        );
    }
}

/// Run walk-forward analysis across a ramping sentiment path.
fn run_walk_forward(
    universe: &Universe,
    policy: &RiskPolicy,
    cap: f64,
    steps: u32,
    windows: u32,
    fg_values: &[u32],
    preset: &str,
) {
    // Prefer the user-supplied ramp; fall back to the default regime ramp.
    let fear_greed_path: Vec<u32> = if fg_values.is_empty() {
        DEFAULT_WF_PATH.to_vec()
    } else {
        fg_values.to_vec()
    };

    let report = walk_forward(
        universe,
        policy.clone(),
        strategy_config(cap, preset),
        WalkForwardConfig {
            windows,
            steps_per_window: steps,
            fear_greed_path,
        },
    );

    println!("Guardrail Alpha — walk-forward ({windows} windows, {steps} steps each)\n");
    println!(
        "{:>7}  {:>10}  {:>12}  {:>12}  {:>12}  {:>12}  {:>7}",
        "window", "fear/greed", "return %", "buy&hold %", "excess %", "max dd %", "trades"
    );
    println!("{}", "-".repeat(86));

    for w in &report.windows {
        println!(
            "{:>7}  {:>10}  {:>12}  {:>12}  {:>12}  {:>12}  {:>7}",
            w.window,
            w.fear_greed,
            w.total_return_pct,
            w.benchmark_return_pct,
            w.excess_return_pct,
            w.max_drawdown_pct,
            w.trades
        );
    }

    println!("{}", "-".repeat(86));
    println!(
        "aggregate: mean excess {} %  worst drawdown {} %  positive windows {}/{}",
        report.mean_excess_pct,
        report.worst_drawdown_pct,
        report.positive_windows,
        report.windows.len()
    );
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let universe = Universe::load(&cli.universe)?;
    let policy_raw = std::fs::read_to_string(&cli.policy)?;
    let policy = RiskPolicy::from_json_str(&policy_raw)?;
    let cap = (to_f64(policy.max_position_pct) - 1.0).max(1.0);

    let fg_values = parse_fear_greed(&cli.fear_greed);

    // Resolve and announce the active preset once, before the per-run loops.
    let base = StrategyConfig {
        max_position_weight_pct: cap,
        min_score_to_enter: 0.55,
        min_score_to_hold: 0.45,
        ..StrategyConfig::default()
    };
    let (_cfg, note) = apply_preset(base, &cli.preset);
    println!("{note}\n");

    if cli.walk_forward {
        run_walk_forward(
            &universe,
            &policy,
            cap,
            cli.steps,
            cli.windows,
            &fg_values,
            &cli.preset,
        );
    } else {
        if fg_values.is_empty() {
            anyhow::bail!("no valid fear_greed values provided");
        }
        run_sweep(&universe, &policy, cap, cli.steps, &fg_values, &cli.preset);
    }

    Ok(())
}
