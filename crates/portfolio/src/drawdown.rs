//! Drawdown tracking. The risk engine reads both total and daily drawdown from
//! here to enforce throttles and the kill switch.

use common::time::utc_day;
use common::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawdownTracker {
    /// All-time peak NAV.
    pub peak_nav_usd: Decimal,
    /// NAV at the start of the current UTC day.
    pub day_open_nav_usd: Decimal,
    /// The UTC day the day-open was captured for.
    pub current_day: String,
}

impl DrawdownTracker {
    pub fn new(starting_nav_usd: Decimal, now_ms: i64) -> Self {
        DrawdownTracker {
            peak_nav_usd: starting_nav_usd,
            day_open_nav_usd: starting_nav_usd,
            current_day: utc_day(now_ms),
        }
    }

    /// Update peaks and roll the day boundary. Call once per NAV update.
    pub fn observe(&mut self, nav_usd: Decimal, now_ms: i64) {
        if nav_usd > self.peak_nav_usd {
            self.peak_nav_usd = nav_usd;
        }
        let today = utc_day(now_ms);
        if today != self.current_day {
            self.current_day = today;
            self.day_open_nav_usd = nav_usd;
        }
    }

    /// Total drawdown from peak, as a positive percent.
    pub fn total_drawdown_pct(&self, nav_usd: Decimal) -> Decimal {
        if self.peak_nav_usd.is_zero() {
            return Decimal::ZERO;
        }
        ((self.peak_nav_usd - nav_usd) / self.peak_nav_usd * Decimal::from(100)).max(Decimal::ZERO)
    }

    /// Drawdown since the start of the current UTC day, as a positive percent.
    pub fn daily_drawdown_pct(&self, nav_usd: Decimal) -> Decimal {
        if self.day_open_nav_usd.is_zero() {
            return Decimal::ZERO;
        }
        ((self.day_open_nav_usd - nav_usd) / self.day_open_nav_usd * Decimal::from(100))
            .max(Decimal::ZERO)
    }
}
