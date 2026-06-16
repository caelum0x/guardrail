//! Error type for the position-sizer crate.

use std::fmt;

/// Errors returned by position sizing routines.
///
/// All sizing functions validate their inputs at the boundary and return a
/// descriptive [`SizingError`] rather than panicking or producing NaN/Inf.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SizingError {
    /// A value that must be strictly positive was zero or negative.
    NotPositive {
        /// Name of the offending parameter.
        field: &'static str,
        /// Human-readable representation of the supplied value.
        value: String,
    },
    /// A value that must be non-negative was negative.
    Negative {
        /// Name of the offending parameter.
        field: &'static str,
        /// Human-readable representation of the supplied value.
        value: String,
    },
    /// A fraction/multiplier fell outside its allowed range.
    OutOfRange {
        /// Name of the offending parameter.
        field: &'static str,
        /// Human-readable representation of the supplied value.
        value: String,
        /// Inclusive lower bound.
        min: f64,
        /// Inclusive upper bound.
        max: f64,
    },
    /// A non-finite (NaN or infinite) value was supplied.
    NotFinite {
        /// Name of the offending parameter.
        field: &'static str,
    },
    /// An empty asset set was supplied where at least one asset is required.
    EmptyInput {
        /// Name of the offending parameter.
        field: &'static str,
    },
}

impl fmt::Display for SizingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SizingError::NotPositive { field, value } => {
                write!(f, "`{field}` must be > 0, got {value}")
            }
            SizingError::Negative { field, value } => {
                write!(f, "`{field}` must be >= 0, got {value}")
            }
            SizingError::OutOfRange {
                field,
                value,
                min,
                max,
            } => write!(
                f,
                "`{field}` must be within [{min}, {max}], got {value}"
            ),
            SizingError::NotFinite { field } => {
                write!(f, "`{field}` must be a finite number")
            }
            SizingError::EmptyInput { field } => {
                write!(f, "`{field}` must contain at least one element")
            }
        }
    }
}

impl std::error::Error for SizingError {}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, SizingError>;

/// Validate that `x` is finite, returning [`SizingError::NotFinite`] otherwise.
pub(crate) fn ensure_finite(field: &'static str, x: f64) -> Result<()> {
    if x.is_finite() {
        Ok(())
    } else {
        Err(SizingError::NotFinite { field })
    }
}

/// Validate that `x` is finite and strictly positive.
pub(crate) fn ensure_positive(field: &'static str, x: f64) -> Result<()> {
    ensure_finite(field, x)?;
    if x > 0.0 {
        Ok(())
    } else {
        Err(SizingError::NotPositive {
            field,
            value: x.to_string(),
        })
    }
}

/// Validate that `x` is finite and non-negative.
#[allow(dead_code)] // boundary helper kept for completeness; covered by tests
pub(crate) fn ensure_non_negative(field: &'static str, x: f64) -> Result<()> {
    ensure_finite(field, x)?;
    if x >= 0.0 {
        Ok(())
    } else {
        Err(SizingError::Negative {
            field,
            value: x.to_string(),
        })
    }
}

/// Validate that `x` is finite and within the inclusive range `[min, max]`.
pub(crate) fn ensure_in_range(
    field: &'static str,
    x: f64,
    min: f64,
    max: f64,
) -> Result<()> {
    ensure_finite(field, x)?;
    if x >= min && x <= max {
        Ok(())
    } else {
        Err(SizingError::OutOfRange {
            field,
            value: x.to_string(),
            min,
            max,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_rejects_zero_and_negative() {
        assert!(ensure_positive("x", 1.0).is_ok());
        assert!(matches!(
            ensure_positive("x", 0.0),
            Err(SizingError::NotPositive { .. })
        ));
        assert!(matches!(
            ensure_positive("x", -1.0),
            Err(SizingError::NotPositive { .. })
        ));
    }

    #[test]
    fn non_negative_accepts_zero() {
        assert!(ensure_non_negative("x", 0.0).is_ok());
        assert!(matches!(
            ensure_non_negative("x", -0.5),
            Err(SizingError::Negative { .. })
        ));
    }

    #[test]
    fn range_bounds_are_inclusive() {
        assert!(ensure_in_range("x", 0.0, 0.0, 1.0).is_ok());
        assert!(ensure_in_range("x", 1.0, 0.0, 1.0).is_ok());
        assert!(matches!(
            ensure_in_range("x", 1.5, 0.0, 1.0),
            Err(SizingError::OutOfRange { .. })
        ));
    }

    #[test]
    fn finite_rejects_nan_and_inf() {
        assert!(matches!(
            ensure_finite("x", f64::NAN),
            Err(SizingError::NotFinite { .. })
        ));
        assert!(matches!(
            ensure_positive("x", f64::INFINITY),
            Err(SizingError::NotFinite { .. })
        ));
    }

    #[test]
    fn display_is_descriptive() {
        let e = SizingError::OutOfRange {
            field: "kelly_fraction",
            value: "2".to_string(),
            min: 0.0,
            max: 1.0,
        };
        assert_eq!(
            e.to_string(),
            "`kelly_fraction` must be within [0, 1], got 2"
        );
    }
}
