//! NAV time-series helper used by the API and reports.

use common::time::now_ms;
use common::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavPoint {
    pub timestamp_ms: i64,
    pub nav_usd: Decimal,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NavHistory {
    pub points: Vec<NavPoint>,
}

impl NavHistory {
    pub fn record(&mut self, nav_usd: Decimal) {
        self.points.push(NavPoint {
            timestamp_ms: now_ms(),
            nav_usd,
        });
    }

    pub fn latest(&self) -> Option<&NavPoint> {
        self.points.last()
    }
}
