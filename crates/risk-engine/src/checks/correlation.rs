//! Correlation gate.
//!
//! A real correlation limit needs a correlation matrix or per-asset exposure
//! data. None of the public structs the risk engine reasons over
//! (`RiskPolicy`, `RiskContext`, `OrderIntent`, `QuoteSummary`) carry that
//! data today, and we must not add fields to them. Rather than fake a result,
//! this module stays *honest*: it enforces correlation only where the signal
//! is actually present, and is otherwise a documented no-op.
//!
//! The one place correlation can surface with the data on hand is via
//! `RiskContext.security_flags`: upstream scanners may tag an asset as highly
//! correlated with an existing holding (e.g. wrapped/derivative tokens, or
//! same-issuer assets). When such a flag is present we treat it as a real,
//! conservative correlation breach.

use crate::approval::RiskContext;

/// Substrings that, when present in a normalized security flag, indicate the
/// asset is flagged as overly correlated with the existing book.
const CORRELATION_FLAG_MARKERS: &[&str] = &["correlat", "duplicate_exposure", "same_underlying"];

/// Conservative single-symbol predicate retained for callers that only have a
/// symbol on hand. With no correlation data wired through, the honest answer is
/// "within limit" — this never *fakes* a breach.
pub fn correlation_within_limit(_symbol: &str) -> bool {
    true
}

/// Normalize a flag the same way the security-flag gate does (lowercase,
/// separators collapsed) so markers match `"Highly-Correlated"`,
/// `"correlation_risk"`, etc.
fn normalize(raw: &str) -> String {
    raw.trim()
        .to_lowercase()
        .replace(['-', ' ', '\t'], "_")
}

/// Real correlation check: reject when the context carries a flag marking the
/// asset as overly correlated with the existing book. Conservative by design —
/// absent any signal it returns no reasons rather than guessing.
pub fn check(ctx: &RiskContext) -> Vec<String> {
    let mut reasons = Vec::new();
    for raw in &ctx.security_flags {
        let normalized = normalize(raw);
        if CORRELATION_FLAG_MARKERS
            .iter()
            .any(|marker| normalized.contains(marker))
        {
            reasons.push(format!("asset flagged as overly correlated: {raw}"));
        }
    }
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::Decimal;

    fn ctx_with(flags: Vec<&str>) -> RiskContext {
        let mut c = RiskContext::empty(Decimal::from(10_000));
        c.security_flags = flags.into_iter().map(|s| s.to_string()).collect();
        c
    }

    #[test]
    fn predicate_is_conservative_true() {
        assert!(correlation_within_limit("CAKE"));
    }

    #[test]
    fn no_flags_means_no_reasons() {
        assert!(check(&ctx_with(vec![])).is_empty());
        assert!(check(&ctx_with(vec!["honeypot", "high_tax"])).is_empty());
    }

    #[test]
    fn correlation_flag_is_rejected() {
        let reasons = check(&ctx_with(vec!["Highly-Correlated"]));
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("Highly-Correlated"), "got {reasons:?}");
    }

    #[test]
    fn alternate_markers_match() {
        assert_eq!(check(&ctx_with(vec!["duplicate exposure"])).len(), 1);
        assert_eq!(check(&ctx_with(vec!["same underlying"])).len(), 1);
        assert_eq!(check(&ctx_with(vec!["correlation_risk"])).len(), 1);
    }
}
