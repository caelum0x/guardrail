//! Positions panel model.
//!
//! Summarizes the current target portfolio / holdings from the run report:
//! per-symbol weights plus aggregate statistics (holding count, total value,
//! total weight, and the largest single position). The raw per-symbol rows
//! continue to live on [`crate::report::RunReport`]; this module adds the
//! aggregate summary that sits above them and degrades gracefully when no
//! positions are present.

use crate::report::Position;

/// Aggregate summary of the portfolio's holdings.
#[derive(Debug, Clone)]
pub struct PositionsSummary {
    pub available: bool,
    pub count: usize,
    /// Sum of position values, formatted to two decimals (or a placeholder).
    pub total_value_usd: String,
    /// Sum of position weights, formatted to two decimals (or a placeholder).
    pub total_weight_pct: String,
    /// Symbol of the largest position by weight, or a placeholder.
    pub top_symbol: String,
    /// Weight of the largest position, formatted, or a placeholder.
    pub top_weight_pct: String,
}

impl PositionsSummary {
    /// Placeholder used when no positions are available.
    pub fn empty() -> Self {
        Self {
            available: false,
            count: 0,
            total_value_usd: "—".to_string(),
            total_weight_pct: "—".to_string(),
            top_symbol: "—".to_string(),
            top_weight_pct: "—".to_string(),
        }
    }

    /// Builds the summary from the report's positions. Fields backed by
    /// unparsable numeric strings fall back to placeholders individually.
    pub fn from_positions(positions: &[Position]) -> Self {
        if positions.is_empty() {
            return Self::empty();
        }

        let total_value: f64 = positions
            .iter()
            .filter_map(|p| parse_num(&p.value_usd))
            .sum();
        let total_weight: f64 = positions
            .iter()
            .filter_map(|p| parse_num(&p.weight_pct))
            .sum();

        let top = positions
            .iter()
            .filter_map(|p| parse_num(&p.weight_pct).map(|w| (p, w)))
            .max_by(|(_, a), (_, b)| a.total_cmp(b));

        let (top_symbol, top_weight_pct) = match top {
            Some((position, weight)) => (position.symbol.clone(), format!("{weight:.2}")),
            None => ("—".to_string(), "—".to_string()),
        };

        Self {
            available: true,
            count: positions.len(),
            total_value_usd: format!("{total_value:.2}"),
            total_weight_pct: format!("{total_weight:.2}"),
            top_symbol,
            top_weight_pct,
        }
    }
}

/// Parses a possibly high-precision decimal string into an `f64`, returning
/// `None` for empty or unparsable values (including the `—` placeholder).
fn parse_num(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed == "—" {
        return None;
    }
    trimmed.parse::<f64>().ok()
}
