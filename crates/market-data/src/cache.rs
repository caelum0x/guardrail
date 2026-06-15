//! A trivial in-memory last-snapshot cache. The agent keeps the most recent
//! snapshot here so health checks and the API can read it without a refetch.

use crate::snapshot::MarketSnapshot;
use std::sync::RwLock;

#[derive(Default)]
pub struct SnapshotCache {
    last: RwLock<Option<MarketSnapshot>>,
}

impl SnapshotCache {
    pub fn new() -> Self {
        SnapshotCache::default()
    }

    pub fn put(&self, snap: MarketSnapshot) {
        if let Ok(mut guard) = self.last.write() {
            *guard = Some(snap);
        }
    }

    pub fn get(&self) -> Option<MarketSnapshot> {
        self.last.read().ok().and_then(|g| g.clone())
    }
}
