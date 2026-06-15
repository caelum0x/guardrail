//! Market-regime classifier. Routes the whole strategy: which assets to favour
//! and how much stable reserve to hold.

use common::decimal::to_f64;
use market_data::RegimeInputs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketRegime {
    RiskOn,
    RiskOff,
    Chop,
    Breakout,
}

impl MarketRegime {
    /// Multiplier applied to position sizing in this regime.
    pub fn exposure_multiplier(&self) -> f64 {
        match self {
            MarketRegime::RiskOn => 1.0,
            MarketRegime::Breakout => 1.1,
            MarketRegime::Chop => 0.5,
            MarketRegime::RiskOff => 0.2,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MarketRegime::RiskOn => "risk_on",
            MarketRegime::RiskOff => "risk_off",
            MarketRegime::Chop => "chop",
            MarketRegime::Breakout => "breakout",
        }
    }
}

/// Classify the regime from market breadth, sentiment, and median return.
pub fn classify(inputs: &RegimeInputs) -> MarketRegime {
    let breadth = to_f64(inputs.breadth_pct);
    let median = to_f64(inputs.median_24h_return);
    let fg = inputs.fear_greed as f64;

    // Strong, broad, well-bid advance.
    if breadth >= 65.0 && median > 2.0 && fg >= 60.0 {
        return MarketRegime::Breakout;
    }
    // Healthy risk appetite.
    if breadth >= 55.0 && fg >= 50.0 {
        return MarketRegime::RiskOn;
    }
    // Fearful, broadly declining.
    if breadth <= 40.0 || fg <= 30.0 || median < -2.0 {
        return MarketRegime::RiskOff;
    }
    // Everything else: directionless.
    MarketRegime::Chop
}
