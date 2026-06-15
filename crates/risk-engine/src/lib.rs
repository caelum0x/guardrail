//! Risk enforcement for Guardrail Alpha.
//!
//! The risk engine is the authority boundary between strategy intent and
//! execution. Orders that do not receive an approval here must never reach TWAK.

pub mod approval;
pub mod audit;
pub mod checks;
pub mod kill_switch;
pub mod policy;
pub mod sizing;
pub mod throttle;

pub use approval::{ApprovedOrder, RiskContext, RiskDecision, RiskEngine};
pub use policy::{DailyTradeRequirement, RiskPolicy};
