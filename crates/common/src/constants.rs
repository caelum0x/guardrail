//! System-wide constants. Anything that must match an on-chain or competition
//! value lives here so there is a single source of truth.

/// BSC mainnet chain id.
pub const BSC_CHAIN_ID: u64 = 56;

/// Track 1 competition contract address.
pub const COMPETITION_CONTRACT: &str = "0x212c61b9b72c95d95bf29cf032f5e5635629aed5";

/// Canonical stable symbol used as the portfolio reserve / quote currency.
pub const RESERVE_SYMBOL: &str = "USDT";

/// Default base currency for accounting.
pub const BASE_CURRENCY: &str = "USD";

/// Maximum age of a market snapshot before it is considered stale (ms).
pub const MAX_SNAPSHOT_AGE_MS: i64 = 5 * 60 * 1000;
