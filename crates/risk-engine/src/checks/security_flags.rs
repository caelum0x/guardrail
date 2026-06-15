//! Security-flag gate.
//!
//! `RiskContext.security_flags` carries token-safety signals attached to the
//! asset that is gaining (buy) or being trimmed (sell) exposure. These come
//! from upstream scanners (honeypot detectors, contract verifiers, tax
//! analysers, etc.). This check turns those raw signals into hard rejections.
//!
//! Two layers of protection:
//!
//! 1. **Disqualifying flags** — any single occurrence of a known-critical flag
//!    (e.g. `honeypot`, `blacklist`, `mintable`) rejects the order outright. A
//!    honeypot can never be exited, so one is one too many.
//! 2. **Aggregate threshold** — even "soft" flags add up. Once the count of
//!    non-empty flags reaches [`MAX_TOTAL_FLAGS`], the asset is treated as too
//!    risky regardless of which individual flags are present.

use crate::approval::RiskContext;

/// Flags that reject the order on their own, regardless of how many others are
/// present. Matched case-insensitively against a normalized form of each flag
/// (lowercased, with `-`/spaces collapsed to `_`).
const DISQUALIFYING_FLAGS: &[&str] = &[
    "honeypot",
    "honey_pot",
    "blacklist",
    "blacklisted",
    "mintable",
    "proxy_upgradeable",
    "self_destruct",
    "selfdestruct",
    "hidden_owner",
    "trading_disabled",
    "cannot_sell",
    "rug_pull",
    "rugpull",
    "fake_token",
    "unverified_contract",
];

/// Maximum number of (non-empty) security flags tolerated in aggregate before
/// the asset is rejected, even when none are individually disqualifying.
const MAX_TOTAL_FLAGS: usize = 3;

/// Normalize a raw flag string for comparison: trim, lowercase, and collapse
/// `-` / whitespace runs into single underscores so `"Honey-Pot"`,
/// `"honey pot"` and `"honeypot"` are all comparable.
fn normalize_flag(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut last_was_sep = false;
    for ch in raw.trim().chars() {
        if ch == '-' || ch == '_' || ch.is_whitespace() {
            if !out.is_empty() && !last_was_sep {
                out.push('_');
                last_was_sep = true;
            }
        } else {
            out.extend(ch.to_lowercase());
            last_was_sep = false;
        }
    }
    // Strip a trailing separator if the input ended with one.
    if out.ends_with('_') {
        out.pop();
    }
    out
}

/// True if a normalized flag is on the hard-disqualify list.
fn is_disqualifying(normalized: &str) -> bool {
    DISQUALIFYING_FLAGS.contains(&normalized)
}

pub fn check(ctx: &RiskContext) -> Vec<String> {
    let mut reasons = Vec::new();

    // Count only meaningful (non-empty after normalization) flags.
    let mut meaningful = 0usize;

    for raw in &ctx.security_flags {
        let normalized = normalize_flag(raw);
        if normalized.is_empty() {
            continue;
        }
        meaningful += 1;

        if is_disqualifying(&normalized) {
            // Preserve the original flag text in the reason so honeypots and the
            // like are clearly identifiable in the audit trail.
            reasons.push(format!("disqualifying security flag present: {raw}"));
        }
    }

    // Aggregate-risk rejection: too many soft flags is itself disqualifying.
    if reasons.is_empty() && meaningful > MAX_TOTAL_FLAGS {
        reasons.push(format!(
            "asset carries {meaningful} security flags, exceeding the limit of {MAX_TOTAL_FLAGS}"
        ));
    }

    reasons
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::RiskContext;
    use common::Decimal;

    fn ctx_with(flags: Vec<&str>) -> RiskContext {
        let mut ctx = RiskContext::empty(Decimal::from(10_000));
        ctx.security_flags = flags.into_iter().map(|s| s.to_string()).collect();
        ctx
    }

    #[test]
    fn clean_asset_passes() {
        assert!(check(&ctx_with(vec![])).is_empty());
    }

    #[test]
    fn empty_or_whitespace_flags_are_ignored() {
        assert!(check(&ctx_with(vec!["", "   ", "\t"])).is_empty());
    }

    #[test]
    fn honeypot_is_rejected_and_named() {
        let reasons = check(&ctx_with(vec!["honeypot"]));
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("honeypot"), "got {reasons:?}");
    }

    #[test]
    fn disqualifying_flag_matched_case_and_separator_insensitively() {
        let reasons = check(&ctx_with(vec!["Honey-Pot"]));
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("Honey-Pot"));

        let reasons = check(&ctx_with(vec!["HONEY POT"]));
        assert_eq!(reasons.len(), 1);
    }

    #[test]
    fn a_few_soft_flags_pass() {
        // Below the aggregate threshold and none disqualifying.
        let reasons = check(&ctx_with(vec!["high_tax", "low_holders"]));
        assert!(reasons.is_empty(), "got {reasons:?}");
    }

    #[test]
    fn too_many_soft_flags_are_rejected() {
        let reasons = check(&ctx_with(vec!["a", "b", "c", "d"]));
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("exceeding the limit"), "got {reasons:?}");
    }

    #[test]
    fn disqualifying_flag_takes_precedence_over_aggregate() {
        // Many flags including a hard one: report the hard flag, not the count.
        let reasons = check(&ctx_with(vec!["a", "b", "c", "blacklist", "d"]));
        assert!(
            reasons.iter().any(|r| r.contains("blacklist")),
            "got {reasons:?}"
        );
        assert!(
            !reasons.iter().any(|r| r.contains("exceeding the limit")),
            "aggregate message should be suppressed: {reasons:?}"
        );
    }

    #[test]
    fn multiple_disqualifying_flags_each_reported() {
        let reasons = check(&ctx_with(vec!["honeypot", "mintable"]));
        assert_eq!(reasons.len(), 2);
    }
}
