//! Prompt that asks the LLM to summarize a daily performance report.
//!
//! The figures are computed upstream from immutable event logs. The LLM only
//! writes human-readable commentary; it never recomputes or alters them.

use crate::prompts::PromptBuilder;

/// Numeric inputs for a daily report summary.
///
/// Values are pre-computed by deterministic code; the LLM only narrates them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DailyReportStats {
    /// Net asset value at the close of the day.
    pub nav: f64,
    /// Profit and loss for the day (may be negative).
    pub pnl: f64,
    /// Peak-to-trough drawdown as a percentage in `[0, 100]`.
    pub drawdown_pct: f64,
    /// Number of trades executed during the day.
    pub trade_count: u32,
}

/// Build a deterministic prompt asking the LLM to summarize a daily report.
///
/// The prompt presents pre-computed figures and asks for a concise,
/// judge-facing narrative. The model must not recompute or change the numbers.
#[must_use]
pub fn build_report_prompt(stats: &DailyReportStats) -> String {
    let figures = format!(
        "NAV: {:.2}\nPnL: {:.2}\nMax drawdown: {:.2}%\nTrades executed: {}",
        stats.nav, stats.pnl, stats.drawdown_pct, stats.trade_count,
    );

    PromptBuilder::new()
        .section(
            "Task",
            "Write a concise daily summary for a non-technical reviewer using the figures below.",
        )
        .section("Figures", &figures)
        .section(
            "Reminder",
            "These figures are final and computed from immutable logs. \
             Summarize them; do not recompute or change any number.",
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DailyReportStats {
        DailyReportStats {
            nav: 1_050_000.0,
            pnl: -2_500.0,
            drawdown_pct: 3.25,
            trade_count: 7,
        }
    }

    #[test]
    fn includes_all_figures() {
        let prompt = build_report_prompt(&sample());
        assert!(prompt.contains("1050000.00"));
        assert!(prompt.contains("-2500.00"));
        assert!(prompt.contains("3.25%"));
        assert!(prompt.contains("Trades executed: 7"));
    }

    #[test]
    fn instructs_no_recompute() {
        let prompt = build_report_prompt(&sample());
        assert!(prompt.contains("do not recompute"));
    }
}
