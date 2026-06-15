//! Tests for time helpers.

use common::time::{from_ms, now_ms, utc_day};

#[test]
fn now_ms_is_positive() {
    assert!(now_ms() > 0);
}

#[test]
fn utc_day_formats_known_epoch_ms() {
    // 2021-01-01T00:00:00Z = 1609459200000 ms.
    assert_eq!(utc_day(1_609_459_200_000), "2021-01-01");
}

#[test]
fn utc_day_formats_yyyy_mm_dd_shape() {
    let day = utc_day(now_ms());
    // YYYY-MM-DD is 10 chars.
    assert_eq!(day.len(), 10);
    let parts: Vec<&str> = day.split('-').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0].len(), 4);
    assert_eq!(parts[1].len(), 2);
    assert_eq!(parts[2].len(), 2);
}

#[test]
fn from_ms_round_trips_a_day() {
    // Midnight UTC on a known day round-trips to the same calendar day.
    let ms = 1_609_459_200_000; // 2021-01-01T00:00:00Z
    let dt = from_ms(ms);
    assert_eq!(dt.timestamp_millis(), ms);
    assert_eq!(utc_day(dt.timestamp_millis()), "2021-01-01");
}
