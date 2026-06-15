//! Time helpers. The system works in UTC milliseconds internally and RFC3339
//! strings at storage boundaries.

use chrono::{DateTime, Utc};

/// Current time in epoch milliseconds.
pub fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

/// Current time as an RFC3339 string.
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

/// Convert epoch milliseconds to a UTC datetime.
pub fn from_ms(ms: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(ms).unwrap_or_else(Utc::now)
}

/// The UTC calendar day (YYYY-MM-DD) for a given epoch-ms timestamp.
pub fn utc_day(ms: i64) -> String {
    from_ms(ms).format("%Y-%m-%d").to_string()
}
