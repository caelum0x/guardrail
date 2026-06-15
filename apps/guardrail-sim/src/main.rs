//! Guardrail Alpha scenario sweep.
//!
//! Runs the real backtest engine across a range of Fear & Greed inputs to show
//! how the regime-routed strategy adapts: how exposure, return, and drawdown
//! shift from fearful (defensive) to greedy (constructive) markets. Reuses the
//! production strategy + risk + portfolio path — only the sentiment input varies.
//!
//! Modes:
//!   * default          — sentiment sweep at the selected preset
//!   * `--walk-forward` — a sequence of windows whose sentiment ramps across regimes
//!   * `--compare-presets` — the sweep run for every preset, ranked by mean excess
//!
//! `--json` emits machine-readable output for any mode.

mod cli;
mod output;
mod preset;
mod sweep;

use backtester::{walk_forward, WalkForwardConfig};
use clap::Parser;
use common::decimal::to_f64;
use market_data::Universe;
use risk_engine::RiskPolicy;

use crate::cli::Cli;

/// Default sentiment ramp for walk-forward when no `--fear-greed` is supplied:
/// fearful -> neutral -> greedy -> cooling off.
const DEFAULT_WF_PATH: [u32; 6] = [25, 40, 55, 70, 85, 60];

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let universe = Universe::load(&cli.universe)?;
    let policy_raw = std::fs::read_to_string(&cli.policy)?;
    let policy = RiskPolicy::from_json_str(&policy_raw)?;
    let cap = (to_f64(policy.max_position_pct) - 1.0).max(1.0);
    let fg_values = cli.fear_greed_values();

    // Cross-preset comparison: its own dispatch, no single active preset.
    if cli.compare_presets {
        if fg_values.is_empty() {
            anyhow::bail!("no valid fear_greed values provided");
        }
        let summaries =
            sweep::compare_presets(&universe, &policy, cap, cli.steps, &fg_values, cli.starting_usd);
        if cli.json {
            output::compare_json(&summaries);
        } else {
            output::compare_table(&summaries);
        }
        return Ok(());
    }

    let (cfg, note) = preset::strategy_config(cap, &cli.preset);
    if !cli.json {
        println!("{note}\n");
    }

    if cli.walk_forward {
        let fear_greed_path = if fg_values.is_empty() {
            DEFAULT_WF_PATH.to_vec()
        } else {
            fg_values
        };
        let report = walk_forward(
            &universe,
            policy.clone(),
            cfg,
            WalkForwardConfig {
                windows: cli.windows,
                steps_per_window: cli.steps,
                fear_greed_path,
            },
        );
        if cli.json {
            output::walk_forward_json(&report, cli.steps);
        } else {
            output::walk_forward_table(&report, cli.steps);
        }
    } else {
        if fg_values.is_empty() {
            anyhow::bail!("no valid fear_greed values provided");
        }
        let rows = sweep::run_sweep(&universe, &policy, &cfg, cli.steps, &fg_values, cli.starting_usd);
        if cli.json {
            output::sweep_json(&rows, cli.steps);
        } else {
            output::sweep_table(&rows, cli.steps);
        }
    }

    Ok(())
}
