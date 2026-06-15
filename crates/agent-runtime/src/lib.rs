//! Autonomous runtime loops for data, strategy, risk, reconciliation, reports,
//! and health.

pub mod daily_trade_loop;
pub mod data_loop;
pub mod error;
pub mod reconciliation_loop;
pub mod report_loop;
pub mod runtime;
pub mod scheduler;
pub mod shutdown;
pub mod state_machine;
pub mod trading_loop;

pub use runtime::{AgentRuntime, RuntimeConfig};
