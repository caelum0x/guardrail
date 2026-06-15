//! Portfolio accounting: holdings, NAV, PnL, drawdown, and exposure.
//!
//! This is the system's book of record. The risk engine reads drawdown and
//! reserve levels from here; execution reconciles against it after each fill.

pub mod drawdown;
pub mod exposure;
pub mod holding;
pub mod nav;
pub mod pnl;
pub mod portfolio_state;
pub mod reconciliation;
pub mod trade_accounting;

pub use drawdown::DrawdownTracker;
pub use holding::Holding;
pub use portfolio_state::PortfolioState;
