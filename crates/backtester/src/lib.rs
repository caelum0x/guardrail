//! Rust-native backtesting harness.

pub mod benchmark;
pub mod engine;
pub mod gas;
pub mod historical_data;
pub mod metrics;
pub mod report;
pub mod simulator;
pub mod slippage;
pub mod synthetic;
pub mod walk_forward;

pub use engine::{run_backtest, BacktestConfig, BacktestRun};
pub use metrics::BacktestMetrics;
pub use walk_forward::{walk_forward, WalkForwardConfig, WalkForwardReport, WindowResult};
