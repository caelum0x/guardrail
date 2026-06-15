//! Normalization primitives shared by every feature.

/// Squash any real number into 0..1 with a logistic curve centered at 0.
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Clamp to the unit interval.
pub fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// Linear min-max scale of `x` within `[lo, hi]` into 0..1 (clamped).
pub fn min_max(x: f64, lo: f64, hi: f64) -> f64 {
    if (hi - lo).abs() < f64::EPSILON {
        return 0.5;
    }
    clamp01((x - lo) / (hi - lo))
}
