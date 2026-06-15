//! Alert types produced by the watchdog checks.

use std::fmt;

/// Severity of a monitor alert, ordered from least to most urgent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational; part of the severity scale and handled by the logger
    /// even though current checks only emit warnings and criticals.
    #[allow(dead_code)]
    Info,
    Warning,
    Critical,
}

impl Severity {
    /// Stable uppercase label for log lines.
    pub fn label(self) -> &'static str {
        match self {
            Severity::Info => "INFO",
            Severity::Warning => "WARNING",
            Severity::Critical => "CRITICAL",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// A single alert raised by a check over a [`RunReport`](crate::report::RunReport).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorAlert {
    pub severity: Severity,
    pub message: String,
}

impl MonitorAlert {
    /// Build a new alert from any displayable message.
    pub fn new(severity: Severity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
        }
    }

    /// Render the alert as a single human-readable line.
    pub fn format(&self) -> String {
        format!("[{}] {}", self.severity.label(), self.message)
    }
}

impl fmt::Display for MonitorAlert {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format())
    }
}
