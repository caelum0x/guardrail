//! Snapshot validation. A snapshot that fails here must not drive trading —
//! the agent treats stale or empty data as a hard stop.

use crate::snapshot::MarketSnapshot;
use common::constants::MAX_SNAPSHOT_AGE_MS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotValidity {
    Ok,
    Stale { age_ms: i64 },
    Empty,
    NoPrices,
}

impl SnapshotValidity {
    pub fn is_ok(&self) -> bool {
        matches!(self, SnapshotValidity::Ok)
    }
}

/// Validate a snapshot for trading use.
pub fn validate(snap: &MarketSnapshot) -> SnapshotValidity {
    if snap.assets.is_empty() {
        return SnapshotValidity::Empty;
    }
    let age = snap.age_ms();
    if age > MAX_SNAPSHOT_AGE_MS {
        return SnapshotValidity::Stale { age_ms: age };
    }
    let any_price = snap
        .assets
        .iter()
        .any(|a| a.price_usd > common::Decimal::ZERO);
    if !any_price {
        return SnapshotValidity::NoPrices;
    }
    SnapshotValidity::Ok
}
