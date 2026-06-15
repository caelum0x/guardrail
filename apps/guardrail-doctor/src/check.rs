//! The check-result model shared by every preflight check.

/// Outcome severity for a single check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pass,
    Warn,
    Fail,
}

impl Status {
    /// Single-character glyph for the checklist column.
    pub fn mark(self) -> &'static str {
        match self {
            Status::Pass => "\u{2713}", // checkmark
            Status::Warn => "!",
            Status::Fail => "\u{2717}", // cross
        }
    }

    /// Fixed-width text label.
    pub fn label(self) -> &'static str {
        match self {
            Status::Pass => "PASS",
            Status::Warn => "WARN",
            Status::Fail => "FAIL",
        }
    }

    /// Lowercase form used in JSON output.
    pub fn json(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Warn => "warn",
            Status::Fail => "fail",
        }
    }
}

/// The result of running a single named check.
pub struct CheckResult {
    pub name: String,
    pub status: Status,
    pub detail: String,
}

impl CheckResult {
    pub fn new(name: impl Into<String>, status: Status, detail: impl Into<String>) -> Self {
        CheckResult {
            name: name.into(),
            status,
            detail: detail.into(),
        }
    }

    pub fn pass(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(name, Status::Pass, detail)
    }

    pub fn warn(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(name, Status::Warn, detail)
    }

    pub fn fail(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(name, Status::Fail, detail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_marks_are_distinct() {
        assert_ne!(Status::Pass.mark(), Status::Fail.mark());
        assert_ne!(Status::Pass.mark(), Status::Warn.mark());
    }

    #[test]
    fn json_forms_are_lowercase() {
        assert_eq!(Status::Pass.json(), "pass");
        assert_eq!(Status::Warn.json(), "warn");
        assert_eq!(Status::Fail.json(), "fail");
    }
}
