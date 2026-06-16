//! CLI command implementations, grouped by domain.
//!
//! `main.rs` owns argument parsing and dispatch; each submodule here owns the
//! `run_*` logic for one command group. Command runners may use crate-root
//! helpers (`apply_preset`, `strategy_config`, the path constants) and the
//! shared [`crate::util`] helpers.

pub mod agent_surface;
pub mod backtest;
pub mod commerce;
pub mod experiment;
pub mod identity;
pub mod market;
pub mod portfolio;
pub mod quant;
pub mod reporting;
