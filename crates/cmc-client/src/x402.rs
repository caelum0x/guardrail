//! x402 "pay-per-request" support for CMC premium endpoints.
//!
//! The x402 protocol returns HTTP 402 with a set of accepted payment terms; the
//! client retries with an `X-PAYMENT` header carrying a signed payment payload.
//! This module models the terms and payload and renders the header value. The
//! actual signature/settlement is delegated to TWAK (self-custody), so we never
//! hold keys here — we produce the structured payload TWAK signs.

use serde::{Deserialize, Serialize};

/// Env flag that enables x402 paid requests.
pub const X402_ENABLED_ENV: &str = "CMC_X402_ENABLED";
/// Header carrying the payment payload on a retried request.
pub const PAYMENT_HEADER: &str = "X-PAYMENT";
/// Header servers use to advertise accepted terms (base for 402 responses).
pub const ACCEPTS_HEADER: &str = "X-PAYMENT-ACCEPTS";

/// Attach a signed payment payload to an outgoing request via the `X-PAYMENT`
/// header.
///
/// Note: [`crate::client::CmcRestClient`] already implements the full 402 retry
/// loop (`pay_and_retry`): on an HTTP 402 it parses the advertised
/// [`PaymentRequirements`], builds a [`PaymentPayload`], and replays the request
/// with this header set. This helper exposes the same header-setting step for
/// callers that construct requests directly (e.g. premium MCP/REST surfaces).
pub fn attach_payment(
    builder: reqwest::RequestBuilder,
    payment: &PaymentPayload,
) -> reqwest::RequestBuilder {
    builder.header(PAYMENT_HEADER, payment.header_value())
}

/// Whether x402 paid requests are enabled via env.
pub fn is_enabled() -> bool {
    std::env::var(X402_ENABLED_ENV)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Payment terms advertised by the server in a 402 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequirements {
    /// Payment scheme, e.g. "exact".
    pub scheme: String,
    /// Network identifier, e.g. "bsc" / chain id.
    pub network: String,
    /// Smallest-unit amount required.
    pub max_amount_required: String,
    /// Asset contract address to pay in (e.g. USDT on BSC).
    pub asset: String,
    /// Recipient address.
    pub pay_to: String,
    /// Resource being paid for.
    pub resource: String,
}

/// A payment payload sent back in the `X-PAYMENT` header. The `signature` is
/// produced out-of-band by TWAK over the canonical `authorization` JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPayload {
    pub scheme: String,
    pub network: String,
    pub asset: String,
    pub pay_to: String,
    pub amount: String,
    pub from: String,
    /// Hex signature over the authorization, supplied by the wallet/TWAK.
    pub signature: String,
}

impl PaymentPayload {
    /// Build an unsigned payload from server terms and the payer address.
    pub fn from_requirements(req: &PaymentRequirements, from: impl Into<String>) -> Self {
        PaymentPayload {
            scheme: req.scheme.clone(),
            network: req.network.clone(),
            asset: req.asset.clone(),
            pay_to: req.pay_to.clone(),
            amount: req.max_amount_required.clone(),
            from: from.into(),
            signature: String::new(),
        }
    }

    /// Attach a wallet/TWAK signature.
    pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = signature.into();
        self
    }

    /// The canonical JSON a wallet signs (signature field excluded).
    pub fn authorization_json(&self) -> String {
        let unsigned = serde_json::json!({
            "scheme": self.scheme,
            "network": self.network,
            "asset": self.asset,
            "pay_to": self.pay_to,
            "amount": self.amount,
            "from": self.from,
        });
        serde_json::to_string(&unsigned).unwrap_or_default()
    }

    /// The value for the `X-PAYMENT` header (JSON; facilitators accept JSON).
    pub fn header_value(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// True once a signature has been attached.
    pub fn is_signed(&self) -> bool {
        !self.signature.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_requirements() -> PaymentRequirements {
        PaymentRequirements {
            scheme: "exact".to_string(),
            network: "bsc".to_string(),
            max_amount_required: "1000000".to_string(),
            asset: "0x55d398326f99059fF775485246999027B3197955".to_string(),
            pay_to: "0x000000000000000000000000000000000000dEaD".to_string(),
            resource: "https://pro-api.coinmarketcap.com/v1/quotes".to_string(),
        }
    }

    #[test]
    fn is_enabled_reads_env_flag() {
        // SAFETY-of-test: serialize via a unique value; set/unset around the read.
        std::env::set_var(X402_ENABLED_ENV, "1");
        assert!(is_enabled());

        std::env::set_var(X402_ENABLED_ENV, "true");
        assert!(is_enabled());

        std::env::set_var(X402_ENABLED_ENV, "TRUE");
        assert!(is_enabled());

        std::env::set_var(X402_ENABLED_ENV, "0");
        assert!(!is_enabled());

        std::env::set_var(X402_ENABLED_ENV, "no");
        assert!(!is_enabled());

        std::env::remove_var(X402_ENABLED_ENV);
        assert!(!is_enabled());
    }

    #[test]
    fn from_requirements_copies_fields_and_sets_amount_and_from() {
        let req = sample_requirements();
        let payload = PaymentPayload::from_requirements(&req, "0xPayer");

        assert_eq!(payload.scheme, req.scheme);
        assert_eq!(payload.network, req.network);
        assert_eq!(payload.asset, req.asset);
        assert_eq!(payload.pay_to, req.pay_to);
        // amount is sourced from max_amount_required.
        assert_eq!(payload.amount, req.max_amount_required);
        assert_eq!(payload.from, "0xPayer");
        // Unsigned by default.
        assert!(payload.signature.is_empty());
        assert!(!payload.is_signed());
    }

    #[test]
    fn with_signature_marks_payload_signed() {
        let req = sample_requirements();
        let payload = PaymentPayload::from_requirements(&req, "0xPayer");
        assert!(!payload.is_signed());

        let signed = payload.with_signature("0xdeadbeef");
        assert_eq!(signed.signature, "0xdeadbeef");
        assert!(signed.is_signed());
    }

    #[test]
    fn authorization_json_excludes_signature() {
        let req = sample_requirements();
        let signed =
            PaymentPayload::from_requirements(&req, "0xPayer").with_signature("0xdeadbeefcafe");

        let auth = signed.authorization_json();
        assert!(!auth.is_empty());
        // The canonical authorization JSON must not contain the signature.
        assert!(
            !auth.contains("signature"),
            "authorization_json must omit the signature key: {auth}"
        );
        assert!(!auth.contains("0xdeadbeefcafe"));
        // But it must carry the signed economic fields.
        assert!(auth.contains("\"amount\""));
        assert!(auth.contains("\"from\""));
        assert!(auth.contains(&req.max_amount_required));
    }

    #[test]
    fn header_value_round_trips_via_serde() {
        let req = sample_requirements();
        let original =
            PaymentPayload::from_requirements(&req, "0xPayer").with_signature("0xfeedface");

        let header = original.header_value();
        let parsed: PaymentPayload =
            serde_json::from_str(&header).expect("header value must be valid PaymentPayload JSON");

        assert_eq!(parsed.scheme, original.scheme);
        assert_eq!(parsed.network, original.network);
        assert_eq!(parsed.asset, original.asset);
        assert_eq!(parsed.pay_to, original.pay_to);
        assert_eq!(parsed.amount, original.amount);
        assert_eq!(parsed.from, original.from);
        assert_eq!(parsed.signature, original.signature);
        assert!(parsed.is_signed());
    }
}
