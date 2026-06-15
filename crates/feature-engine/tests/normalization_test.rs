//! Tests for the shared normalization primitives.

use feature_engine::normalization::{clamp01, min_max, sigmoid};

#[test]
fn sigmoid_is_bounded_in_unit_interval() {
    for &x in &[-1000.0, -10.0, -1.0, 0.0, 1.0, 10.0, 1000.0] {
        let y = sigmoid(x);
        assert!((0.0..=1.0).contains(&y), "sigmoid({x}) = {y} out of 0..1");
    }
}

#[test]
fn sigmoid_centered_at_half() {
    assert!((sigmoid(0.0) - 0.5).abs() < 1e-12);
}

#[test]
fn sigmoid_is_monotonically_increasing() {
    let xs = [-5.0, -2.0, -0.5, 0.0, 0.5, 2.0, 5.0];
    for pair in xs.windows(2) {
        assert!(
            sigmoid(pair[1]) > sigmoid(pair[0]),
            "sigmoid not increasing between {} and {}",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn clamp01_clamps_below_and_above() {
    assert_eq!(clamp01(-3.0), 0.0);
    assert_eq!(clamp01(0.0), 0.0);
    assert_eq!(clamp01(0.42), 0.42);
    assert_eq!(clamp01(1.0), 1.0);
    assert_eq!(clamp01(5.0), 1.0);
}

#[test]
fn min_max_scales_within_range() {
    assert_eq!(min_max(5.0, 0.0, 10.0), 0.5);
    assert_eq!(min_max(0.0, 0.0, 10.0), 0.0);
    assert_eq!(min_max(10.0, 0.0, 10.0), 1.0);
}

#[test]
fn min_max_clamps_outside_range() {
    assert_eq!(min_max(-5.0, 0.0, 10.0), 0.0);
    assert_eq!(min_max(20.0, 0.0, 10.0), 1.0);
}

#[test]
fn min_max_degenerate_range_returns_half() {
    // lo == hi: nothing to scale across, returns the neutral midpoint.
    assert_eq!(min_max(7.0, 5.0, 5.0), 0.5);
}
