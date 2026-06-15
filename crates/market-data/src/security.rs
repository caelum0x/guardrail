//! Security-signal helpers used by the risk engine's security check.

use crate::snapshot::AssetMarketState;

/// Flags that should block any new exposure outright.
pub const BLOCKING_FLAGS: &[&str] = &["honeypot", "blacklist", "proxy_mintable", "self_destruct"];

/// Is this asset safe enough to take new exposure, given a minimum score?
pub fn is_tradeable(state: &AssetMarketState, min_safety_score: u32) -> bool {
    if state.safety_score < min_safety_score {
        return false;
    }
    !state
        .security_flags
        .iter()
        .any(|f| BLOCKING_FLAGS.contains(&f.as_str()))
}
