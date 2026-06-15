//! Shared domain types for Guardrail Alpha.
//!
//! This crate holds the vocabulary every other crate speaks: assets, money,
//! order intents, and configuration. It contains no business logic and no I/O.

pub mod asset;
pub mod chain;
pub mod config;
pub mod constants;
pub mod decimal;
pub mod error;
pub mod ids;
pub mod money;
pub mod order;
pub mod time;

pub use asset::{Asset, AssetCategory, EligibleAsset};
pub use chain::{Address, ChainId};
pub use config::Settings;
pub use error::{CommonError, Result};
pub use money::Money;
pub use order::{OrderIntent, OrderSide, QuoteSummary, TargetPosition};

pub use rust_decimal::Decimal;
