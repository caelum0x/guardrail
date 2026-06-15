//! Render a backtest run as a Markdown report for the docs/dashboard.

use crate::engine::BacktestRun;

/// Format a backtest run as a Markdown summary.
pub fn markdown(run: &BacktestRun) -> String {
    let m = &run.metrics;
    format!(
        "# Backtest Report\n\n\
         | Metric | Value |\n\
         |---|---|\n\
         | Steps | {} |\n\
         | Starting NAV | ${} |\n\
         | Final NAV | ${} |\n\
         | Total return | {}% |\n\
         | Buy-and-hold | {}% |\n\
         | Excess (alpha) | {}% |\n\
         | Max drawdown | {}% |\n\
         | Trades | {} |\n\
         | Win rate | {}% |\n\
         | Profit factor | {} |\n\
         | Volatility | {}% |\n\
         | Calmar ratio | {} |\n",
        run.steps,
        run.starting_nav_usd.round_dp(2),
        run.final_nav_usd.round_dp(2),
        m.total_return_pct,
        run.benchmark_return_pct,
        run.excess_return_pct,
        m.max_drawdown_pct,
        m.trade_count,
        m.win_rate_pct,
        m.profit_factor,
        m.volatility_pct,
        m.calmar_ratio,
    )
}
