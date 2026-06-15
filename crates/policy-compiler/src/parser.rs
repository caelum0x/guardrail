//! Natural-language mandate parser.
//!
//! This is the deterministic front-half of the "natural language in, policy
//! out" pipeline. It extracts numeric risk limits and an asset allowlist from a
//! plain-English mandate, starting from the default policy and overriding only
//! what the text specifies. An LLM may *propose* a mandate; this parser and the
//! validator are what actually bind it — the model has no direct authority.

use risk_engine::RiskPolicy;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Normalize a mandate string (trim, collapse internal whitespace).
pub fn normalize_mandate(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse a natural-language mandate into a `RiskPolicy`.
///
/// Recognized phrases (case-insensitive), each followed by a number:
/// total/max drawdown, daily drawdown/loss, max position, new position,
/// stable reserve, slippage, kill switch, trades per day. Asset tickers found
/// in the text populate the allowlist. Unspecified fields keep their defaults.
pub fn parse_mandate(input: &str) -> RiskPolicy {
    let text = input.to_lowercase();
    let mut policy = RiskPolicy::default();

    if let Some(v) = pct_after(
        &text,
        &["total drawdown", "max drawdown", "maximum drawdown"],
    ) {
        policy.max_total_drawdown_pct = v;
    }
    if let Some(v) = pct_after(&text, &["daily drawdown", "daily loss", "max daily"]) {
        policy.max_daily_drawdown_pct = v;
    }
    if let Some(v) = pct_after(&text, &["new position", "max new position"]) {
        policy.max_new_position_pct = v;
    }
    if let Some(v) = pct_after(
        &text,
        &[
            "max position",
            "position size",
            "per position",
            "position cap",
        ],
    ) {
        policy.max_position_pct = v;
    }
    if let Some(v) = pct_after(
        &text,
        &["stable reserve", "stables", "cash reserve", "reserve"],
    ) {
        policy.min_stable_reserve_pct = v;
    }
    if let Some(v) = pct_after(&text, &["slippage"]) {
        policy.max_slippage_pct = v;
    }
    if let Some(v) = pct_after(&text, &["kill switch", "kill-switch", "halt at"]) {
        policy.kill_switch_drawdown_pct = v;
    }
    if let Some(n) = int_after(&text, &["trades per day", "daily trades", "trade per day"]) {
        policy.daily_trade_requirement.min_trades_per_day = n as u32;
        policy.daily_trade_requirement.enabled = true;
    }

    let symbols = extract_symbols(input);
    if !symbols.is_empty() {
        policy.allowed_assets = symbols;
    }

    // Common explicit prohibitions.
    if text.contains("no leverage") || text.contains("without leverage") {
        push_unique(&mut policy.forbidden_actions, "borrow_without_policy");
    }
    if text.contains("don't launch")
        || text.contains("no token launch")
        || text.contains("no launching")
    {
        push_unique(&mut policy.forbidden_actions, "launch_token");
    }

    policy
}

fn push_unique(v: &mut Vec<String>, item: &str) {
    if !v.iter().any(|x| x == item) {
        v.push(item.to_string());
    }
}

/// First percentage/number appearing after any of `keys` in `haystack`.
fn pct_after(haystack: &str, keys: &[&str]) -> Option<Decimal> {
    for key in keys {
        if let Some(pos) = haystack.find(key) {
            if let Some(n) = first_number(&haystack[pos + key.len()..]) {
                return Some(n);
            }
        }
    }
    None
}

/// First integer appearing after any of `keys` (or before, for "2 trades per day").
fn int_after(haystack: &str, keys: &[&str]) -> Option<u64> {
    for key in keys {
        if let Some(pos) = haystack.find(key) {
            let before = &haystack[..pos];
            if let Some(n) = last_number(before) {
                return Some(n.trunc().to_string().parse::<u64>().unwrap_or(0));
            }
            if let Some(n) = first_number(&haystack[pos + key.len()..]) {
                return Some(n.trunc().to_string().parse::<u64>().unwrap_or(0));
            }
        }
    }
    None
}

/// Parse the first decimal number in `s` (skips leading non-digits).
fn first_number(s: &str) -> Option<Decimal> {
    let mut num = String::new();
    let mut started = false;
    for c in s.chars() {
        if c.is_ascii_digit() || (c == '.' && started) {
            num.push(c);
            started = true;
        } else if started {
            break;
        }
    }
    Decimal::from_str(&num).ok()
}

/// Parse the last decimal number in `s`.
fn last_number(s: &str) -> Option<Decimal> {
    let mut nums: Vec<String> = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() || (c == '.' && !cur.is_empty()) {
            cur.push(c);
        } else if !cur.is_empty() {
            nums.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        nums.push(cur);
    }
    nums.last().and_then(|n| Decimal::from_str(n).ok())
}

/// Words that look like tickers but are not assets.
const TICKER_STOPLIST: &[&str] = &["USD", "AND", "THE", "DEX", "AI", "RWA", "BSC", "API"];

/// Extract candidate asset tickers (uppercase 2-6 char tokens) from the text.
pub fn extract_symbols(input: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in input.split(|c: char| !c.is_ascii_alphanumeric()) {
        if raw.len() < 2 || raw.len() > 6 {
            continue;
        }
        if raw
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            && raw.chars().any(|c| c.is_ascii_uppercase())
            && !TICKER_STOPLIST.contains(&raw)
            && !out.iter().any(|x| x == raw)
        {
            out.push(raw.to_string());
        }
    }
    out
}
