//! Regime panel model.
//!
//! Derives the latest market regime classification from the event log (falling
//! back to the run report), together with an exposure multiplier inferred from
//! the regime label. The exposure multiplier expresses how aggressively the
//! agent is allowed to deploy capital in the current regime; it is a display
//! convenience derived from the regime name, not a stored field, so it degrades
//! gracefully when the regime is unknown.

use event_store::{AgentEvent, StoredEvent};

/// Parsed view of the current regime. All fields are display strings (or
/// placeholders) so rendering never deals with missing data.
#[derive(Debug, Clone)]
pub struct RegimePanel {
    pub available: bool,
    /// Raw regime label (e.g. `risk_on`, `neutral`, `risk_off`).
    pub regime: String,
    /// Human-friendly exposure multiplier (e.g. `1.00x`), or a placeholder.
    pub exposure: String,
    /// Where the regime was sourced from (event log vs run report).
    pub source: String,
}

impl RegimePanel {
    /// Placeholder used when no regime can be determined.
    pub fn unavailable() -> Self {
        Self {
            available: false,
            regime: "—".to_string(),
            exposure: "—".to_string(),
            source: "—".to_string(),
        }
    }

    /// Builds the panel preferring the most recent `RegimeClassified` event,
    /// then falling back to the run report's `regime` field.
    ///
    /// `events` are expected newest-first (as returned by `recent`).
    pub fn from_sources(events: &[StoredEvent], report_regime: &str) -> Self {
        if let Some(regime) = latest_regime(events) {
            return Self::from_label(&regime, "event log");
        }
        let trimmed = report_regime.trim();
        if !trimmed.is_empty() && trimmed != "—" {
            return Self::from_label(trimmed, "run report");
        }
        Self::unavailable()
    }

    fn from_label(label: &str, source: &str) -> Self {
        Self {
            available: true,
            regime: label.to_string(),
            exposure: exposure_multiplier(label),
            source: source.to_string(),
        }
    }
}

/// Returns the regime string from the newest `RegimeClassified` event, if any.
fn latest_regime(events: &[StoredEvent]) -> Option<String> {
    events
        .iter()
        .find(|event| matches!(event.event_type, AgentEvent::RegimeClassified))
        .and_then(|event| {
            event
                .payload_json
                .get("regime")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned)
        })
}

/// Maps a regime label to a display-friendly exposure multiplier. Unknown
/// labels yield a placeholder so the panel never claims a false multiplier.
fn exposure_multiplier(label: &str) -> String {
    match label.trim().to_ascii_lowercase().as_str() {
        "risk_on" | "risk-on" | "bull" => "1.00x".to_string(),
        "neutral" | "mixed" => "0.60x".to_string(),
        "risk_off" | "risk-off" | "bear" => "0.25x".to_string(),
        "final" => "1.00x".to_string(),
        _ => "—".to_string(),
    }
}
