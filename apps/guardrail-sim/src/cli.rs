//! Command-line surface for the scenario sweep.

use clap::Parser;

use crate::preset::DEFAULT_PRESET;

pub const DEFAULT_UNIVERSE: &str = "configs/eligible_assets.bsc.json";

#[derive(Debug, Parser)]
#[command(name = "guardrail-sim", about = "Sweep the backtest across sentiment regimes")]
pub struct Cli {
    /// Eligible-asset universe file.
    #[arg(long, default_value = DEFAULT_UNIVERSE)]
    pub universe: String,
    /// Risk policy JSON file.
    #[arg(long, default_value = "configs/risk_policy.paper.json")]
    pub policy: String,
    /// Steps per backtest (per window in walk-forward mode).
    #[arg(long, default_value_t = 60)]
    pub steps: u32,
    /// Comma-separated Fear & Greed values to sweep.
    #[arg(long, default_value = "20,35,50,65,80")]
    pub fear_greed: String,
    /// Starting capital in USD.
    #[arg(long, default_value_t = 10_000)]
    pub starting_usd: u64,
    /// Run walk-forward analysis instead of the sentiment sweep.
    #[arg(long, default_value_t = false)]
    pub walk_forward: bool,
    /// Number of sequential windows for walk-forward mode.
    #[arg(long, default_value_t = 6)]
    pub windows: u32,
    /// Strategy preset to apply (see configs/strategy_presets.json).
    #[arg(long, default_value = DEFAULT_PRESET)]
    pub preset: String,
    /// Run the sentiment sweep for every preset and rank them.
    #[arg(long, default_value_t = false)]
    pub compare_presets: bool,
    /// Emit machine-readable JSON instead of text tables.
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

impl Cli {
    /// Parse the comma-separated `--fear-greed` flag into a list of readings.
    pub fn fear_greed_values(&self) -> Vec<u32> {
        parse_fear_greed(&self.fear_greed)
    }
}

/// Parse a comma-separated list of Fear & Greed readings, ignoring blanks.
pub fn parse_fear_greed(raw: &str) -> Vec<u32> {
    raw.split(',').filter_map(|s| s.trim().parse().ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_skips_blanks() {
        assert_eq!(parse_fear_greed("20, 35 ,50"), vec![20, 35, 50]);
        assert_eq!(parse_fear_greed(""), Vec::<u32>::new());
        assert_eq!(parse_fear_greed("x,40,"), vec![40]);
    }
}
