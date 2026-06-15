//! Tests for the Fear & Greed sentiment mapping.
//!
//! The score rises with greed up to a peak at 75, then tapers as the market
//! becomes overheated. It is always bounded in 0..1.

use feature_engine::sentiment::score_from_fear_greed;

#[test]
fn score_is_bounded_in_unit_interval() {
    for v in (0..=100).step_by(5) {
        let s = score_from_fear_greed(v);
        assert!(
            (0.0..=1.0).contains(&s),
            "score {s} for value {v} out of 0..1"
        );
    }
}

#[test]
fn score_rises_with_greed_up_to_peak() {
    // Monotonically non-decreasing across the rising leg (extreme fear -> peak).
    let rising = [0u32, 15, 30, 45, 60, 75];
    for pair in rising.windows(2) {
        let lo = score_from_fear_greed(pair[0]);
        let hi = score_from_fear_greed(pair[1]);
        assert!(
            hi >= lo,
            "expected non-decreasing on rising leg: f({})={} f({})={}",
            pair[0],
            lo,
            pair[1],
            hi
        );
    }
    // Strictly higher comparing the extremes of the rising leg.
    assert!(score_from_fear_greed(75) > score_from_fear_greed(0));
}

#[test]
fn score_peaks_at_value_75() {
    let peak = score_from_fear_greed(75);
    assert!(
        (peak - 1.0).abs() < 1e-12,
        "expected peak 1.0 at 75, got {peak}"
    );
}

#[test]
fn score_tapers_at_extreme_greed() {
    let peak = score_from_fear_greed(75);
    let hot = score_from_fear_greed(90);
    let extreme = score_from_fear_greed(100);
    assert!(
        hot < peak,
        "expected taper above the peak: f(90)={hot} f(75)={peak}"
    );
    assert!(
        extreme < hot,
        "expected further taper at the top: f(100)={extreme} f(90)={hot}"
    );
}
