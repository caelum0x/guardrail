//! Market-data normalization.
//!
//! Raw CMC JSON stops here. Everything downstream consumes a
//! [`MarketSnapshot`]: a validated, point-in-time view of the eligible
//! universe with returns, liquidity, and security already attached.

pub mod cache;
pub mod candle;
pub mod liquidity;
pub mod market_regime_inputs;
pub mod security;
pub mod snapshot;
pub mod universe;
pub mod validator;

pub use market_regime_inputs::RegimeInputs;
pub use snapshot::{AssetMarketState, GlobalMarketState, MarketSnapshot, SnapshotBuilder};
pub use universe::Universe;

// Re-export the CMC sentiment type so downstream crates have one name for it.
pub use cmc_client::FearGreedSnapshot;
