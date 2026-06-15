//! Text-table and JSON rendering for sweep / walk-forward / compare results.

use backtester::WalkForwardReport;
use serde_json::json;

use crate::sweep::{PresetSummary, SweepRow};

/// Print the sentiment sweep as a table.
pub fn sweep_table(rows: &[SweepRow], steps: u32) {
    println!("Guardrail Alpha — sentiment sweep ({steps} steps each)\n");
    println!(
        "{:>10}  {:>12}  {:>12}  {:>12}  {:>12}  {:>7}",
        "fear/greed", "return %", "buy&hold %", "excess %", "max dd %", "trades"
    );
    println!("{}", "-".repeat(78));
    for r in rows {
        println!(
            "{:>10}  {:>12}  {:>12}  {:>12}  {:>12}  {:>7}",
            r.fear_greed, r.return_pct, r.benchmark_pct, r.excess_pct, r.max_drawdown_pct, r.trades
        );
    }
}

/// Print the sweep as JSON.
pub fn sweep_json(rows: &[SweepRow], steps: u32) {
    let doc = json!({ "mode": "sweep", "steps": steps, "rows": rows });
    println!("{}", serde_json::to_string_pretty(&doc).unwrap_or_default());
}

/// Print the walk-forward report as a per-window table plus an aggregate line.
pub fn walk_forward_table(report: &WalkForwardReport, steps: u32) {
    let windows = report.windows.len();
    println!("Guardrail Alpha — walk-forward ({windows} windows, {steps} steps each)\n");
    println!(
        "{:>7}  {:>10}  {:>12}  {:>12}  {:>12}  {:>12}  {:>7}",
        "window", "fear/greed", "return %", "buy&hold %", "excess %", "max dd %", "trades"
    );
    println!("{}", "-".repeat(86));
    for w in &report.windows {
        println!(
            "{:>7}  {:>10}  {:>12}  {:>12}  {:>12}  {:>12}  {:>7}",
            w.window, w.fear_greed, w.total_return_pct, w.benchmark_return_pct, w.excess_return_pct, w.max_drawdown_pct, w.trades
        );
    }
    println!("{}", "-".repeat(86));
    println!(
        "aggregate: mean excess {} %  worst drawdown {} %  positive windows {}/{}",
        report.mean_excess_pct, report.worst_drawdown_pct, report.positive_windows, windows
    );
}

/// Print the walk-forward report as JSON.
pub fn walk_forward_json(report: &WalkForwardReport, steps: u32) {
    let windows: Vec<_> = report
        .windows
        .iter()
        .map(|w| {
            json!({
                "window": w.window,
                "fear_greed": w.fear_greed,
                "return_pct": w.total_return_pct,
                "benchmark_pct": w.benchmark_return_pct,
                "excess_pct": w.excess_return_pct,
                "max_drawdown_pct": w.max_drawdown_pct,
                "trades": w.trades,
            })
        })
        .collect();
    let doc = json!({
        "mode": "walk_forward",
        "steps": steps,
        "windows": windows,
        "mean_excess_pct": report.mean_excess_pct,
        "worst_drawdown_pct": report.worst_drawdown_pct,
        "positive_windows": report.positive_windows,
    });
    println!("{}", serde_json::to_string_pretty(&doc).unwrap_or_default());
}

/// Print the cross-preset comparison as a ranked table.
pub fn compare_table(summaries: &[PresetSummary]) {
    println!("Guardrail Alpha — preset comparison (ranked by mean excess)\n");
    println!("{:>4}  {:<16}  {:>14}  {:>16}  {:>8}", "rank", "preset", "mean excess %", "worst drawdown %", "trades");
    println!("{}", "-".repeat(66));
    for (i, s) in summaries.iter().enumerate() {
        println!(
            "{:>4}  {:<16}  {:>14}  {:>16}  {:>8}",
            i + 1, s.preset, s.mean_excess_pct, s.worst_drawdown_pct, s.total_trades
        );
    }
    if summaries.is_empty() {
        println!("(no presets found in configs/strategy_presets.json)");
    }
}

/// Print the cross-preset comparison as JSON.
pub fn compare_json(summaries: &[PresetSummary]) {
    let doc = json!({ "mode": "compare_presets", "ranked": summaries });
    println!("{}", serde_json::to_string_pretty(&doc).unwrap_or_default());
}
