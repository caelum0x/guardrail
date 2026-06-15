//! `experiment` subcommands: run a tagged backtest and persist it, then list or
//! compare saved experiments. Records live as JSON under [`EXPERIMENTS_DIR`].

use crate::util::{json_num_fmt, json_str, metric_f64, metric_fmt, now_unix_ms};
use crate::{apply_preset, strategy_config, DEFAULT_UNIVERSE};
use common::decimal::to_f64;
use common::Settings;
use rust_decimal::Decimal;

/// Directory where saved experiment records are written.
const EXPERIMENTS_DIR: &str = "data/experiments";

/// Run a backtest with the given parameters and persist it as a named experiment.
pub fn run_experiment_run(
    tag: &str,
    config: &str,
    steps: u32,
    fear_greed: u32,
    preset: &str,
) -> anyhow::Result<()> {
    if tag.trim().is_empty() {
        anyhow::bail!("--tag must not be empty");
    }

    let settings = Settings::load(config)?;
    let universe = market_data::Universe::load(DEFAULT_UNIVERSE)?;
    let policy_raw = std::fs::read_to_string(&settings.risk.policy_path)?;
    let policy = risk_engine::RiskPolicy::from_json_str(&policy_raw)?;
    let cap = (to_f64(policy.max_position_pct) - 1.0).max(1.0);
    let strat_cfg = apply_preset(strategy_config(&settings, cap), preset);

    let cfg = backtester::BacktestConfig {
        steps,
        starting_usd: Decimal::from(10_000),
        fear_greed,
    };
    let run = backtester::run_backtest(&universe, policy, strat_cfg, cfg);
    let m = &run.metrics;

    let record = serde_json::json!({
        "tag": tag,
        "created_ms": now_unix_ms().to_string(),
        "steps": steps,
        "fear_greed": fear_greed,
        "preset": preset,
        "metrics": {
            "total_return_pct": to_f64(m.total_return_pct),
            "max_drawdown_pct": to_f64(m.max_drawdown_pct),
            "trade_count": m.trade_count,
            "win_rate_pct": to_f64(m.win_rate_pct),
            "profit_factor": to_f64(m.profit_factor),
            "volatility_pct": to_f64(m.volatility_pct),
            "calmar_ratio": to_f64(m.calmar_ratio),
        },
        "benchmark_return_pct": to_f64(run.benchmark_return_pct),
        "excess_return_pct": to_f64(run.excess_return_pct),
        "final_nav_usd": to_f64(run.final_nav_usd),
    });

    std::fs::create_dir_all(EXPERIMENTS_DIR)?;
    let path = format!("{EXPERIMENTS_DIR}/{tag}.json");
    let serialized = serde_json::to_string_pretty(&record)?;
    std::fs::write(&path, serialized)?;

    println!("saved experiment '{tag}' to {path}");
    Ok(())
}

/// Collect all saved experiment records, sorted by tag for stable output.
///
/// Each entry is `(tag, parsed JSON)`. Files that fail to read or parse are
/// skipped with a note so a single bad file does not break the listing.
fn load_experiments() -> anyhow::Result<Vec<(String, serde_json::Value)>> {
    let entries = match std::fs::read_dir(EXPERIMENTS_DIR) {
        Ok(entries) => entries,
        Err(_) => return Ok(Vec::new()),
    };

    let mut records: Vec<(String, serde_json::Value)> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let tag = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(e) => {
                println!("note: skipping '{}' ({e})", path.display());
                continue;
            }
        };
        match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(value) => records.push((tag, value)),
            Err(e) => println!("note: skipping '{}' (parse error: {e})", path.display()),
        }
    }

    records.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(records)
}

/// List all saved experiments with their key metrics (one line each).
pub fn run_experiment_list() -> anyhow::Result<()> {
    let records = load_experiments()?;
    if records.is_empty() {
        println!("no experiments found in {EXPERIMENTS_DIR}/ (run `experiment run --tag <name>`)");
        return Ok(());
    }

    for (tag, value) in &records {
        let preset = json_str(value, "preset");
        let return_pct = metric_fmt(value, "total_return_pct");
        let excess = json_num_fmt(value, "excess_return_pct");
        let max_dd = metric_fmt(value, "max_drawdown_pct");
        let calmar = metric_fmt(value, "calmar_ratio");
        let trades = metric_f64(value, "trade_count")
            .map(|n| format!("{n:.0}"))
            .unwrap_or_else(|| "n/a".to_string());
        println!(
            "{tag:<20} preset={preset:<12} return={return_pct}% excess={excess}% max_dd={max_dd}% calmar={calmar} trades={trades}"
        );
    }
    Ok(())
}

/// Print a Markdown table comparing all saved experiments.
pub fn run_experiment_compare() -> anyhow::Result<()> {
    let records = load_experiments()?;
    if records.is_empty() {
        println!("no experiments found in {EXPERIMENTS_DIR}/ (run `experiment run --tag <name>`)");
        return Ok(());
    }

    println!("# Experiment Comparison");
    println!();
    println!("| Tag | Return % | Excess % | Max DD % | Calmar | Trades |");
    println!("|:----|---------:|---------:|---------:|-------:|-------:|");
    for (tag, value) in &records {
        let return_pct = metric_fmt(value, "total_return_pct");
        let excess = json_num_fmt(value, "excess_return_pct");
        let max_dd = metric_fmt(value, "max_drawdown_pct");
        let calmar = metric_fmt(value, "calmar_ratio");
        let trades = metric_f64(value, "trade_count")
            .map(|n| format!("{n:.0}"))
            .unwrap_or_else(|| "n/a".to_string());
        println!("| {tag} | {return_pct} | {excess} | {max_dd} | {calmar} | {trades} |");
    }
    Ok(())
}
