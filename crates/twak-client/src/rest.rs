//! REST execution surface for the Trust Wallet Agent Kit.
//!
//! `TwakRestClient` speaks to a TWAK REST endpoint over HTTP and maps the
//! responses onto the shared `SwapQuote` / `TwakPortfolio` / `TxReceipt` types.
//! Parsing is deliberately defensive: TWAK responses are navigated through
//! `serde_json::Value` with sane fallbacks rather than rigid `#[derive]`
//! deserialization, so a missing or renamed field degrades gracefully instead
//! of failing the whole call.
//!
//! When the REST venue answers a swap with HTTP 402, the client signs an x402
//! authorization (see [`crate::x402`]) and retries with the payment attached —
//! mirroring the cmc-client settlement pattern, except TWAK holds the keys.

use crate::error::TwakError;
use crate::parse;
use crate::portfolio::TwakPortfolio;
use crate::quote::SwapQuote;
use crate::swap::TxReceipt;
use crate::{x402, TwakExecutor};
use async_trait::async_trait;
use common::{Address, OrderIntent};
use reqwest::StatusCode;
use risk_engine::ApprovedOrder;
use serde_json::{json, Value};

/// Legacy configuration shape retained for callers that build a client from a
/// single URL. Prefer [`TwakRestClient::new`].
#[derive(Debug, Clone)]
pub struct TwakRestConfig {
    pub url: String,
}

/// HTTP client for the TWAK REST execution surface.
#[derive(Clone)]
pub struct TwakRestClient {
    base_url: String,
    http: reqwest::Client,
    autonomous: bool,
}

impl TwakRestClient {
    /// Build a REST client targeting `base_url`. When `autonomous` is true the
    /// client may submit swaps without an out-of-band approval gate.
    pub fn new(base_url: impl Into<String>, autonomous: bool) -> Self {
        TwakRestClient {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
            autonomous,
        }
    }

    /// Whether this client is allowed to self-submit swaps.
    pub fn is_autonomous(&self) -> bool {
        self.autonomous
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    async fn get_json(&self, path: &str) -> Result<Value, TwakError> {
        let resp = self
            .http
            .get(self.url(path))
            .send()
            .await
            .map_err(|e| TwakError::Transport(e.to_string()))?;
        Self::into_json(resp).await
    }

    async fn post_json(&self, path: &str, body: Value) -> Result<Value, TwakError> {
        let resp = self
            .http
            .post(self.url(path))
            .json(&body)
            .send()
            .await
            .map_err(|e| TwakError::Transport(e.to_string()))?;

        // x402: if the venue demands payment, sign and retry once.
        if resp.status() == StatusCode::PAYMENT_REQUIRED {
            return self.post_with_payment(path, body, resp).await;
        }
        Self::into_json(resp).await
    }

    /// Sign an x402 authorization derived from the 402 body and retry the POST
    /// with the `X-PAYMENT` header attached. TWAK holds the keys, so signing
    /// happens here rather than being delegated outward.
    async fn post_with_payment(
        &self,
        path: &str,
        body: Value,
        challenge: reqwest::Response,
    ) -> Result<Value, TwakError> {
        let terms = challenge.text().await.unwrap_or_default();
        let signer = self.base_url.clone();
        let signed = x402::sign_authorization(&terms, &signer);
        let header = serde_json::to_string(&signed).unwrap_or_default();

        let resp = self
            .http
            .post(self.url(path))
            .header("X-PAYMENT", header)
            .json(&body)
            .send()
            .await
            .map_err(|e| TwakError::Transport(e.to_string()))?;
        Self::into_json(resp).await
    }

    async fn into_json(resp: reqwest::Response) -> Result<Value, TwakError> {
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| TwakError::Transport(e.to_string()))?;

        if !status.is_success() {
            return Err(TwakError::Rejected(format!(
                "TWAK REST {status}: {}",
                text.chars().take(200).collect::<String>()
            )));
        }
        serde_json::from_str(&text).map_err(|e| TwakError::Parse(e.to_string()))
    }
}

#[async_trait]
impl TwakExecutor for TwakRestClient {
    async fn wallet_address(&self) -> Result<Address, TwakError> {
        let v = self.get_json("/wallet").await?;
        let addr = parse::wallet_address(&v)
            .ok_or_else(|| TwakError::Parse("no wallet address in /wallet response".into()))?;
        Ok(Address::new(addr))
    }

    async fn portfolio(&self) -> Result<TwakPortfolio, TwakError> {
        let v = self.get_json("/portfolio").await?;
        Ok(parse::portfolio(&v))
    }

    async fn quote_swap(&self, intent: &OrderIntent) -> Result<SwapQuote, TwakError> {
        let body = json!({
            "from_symbol": intent.from_symbol,
            "to_symbol": intent.to_symbol,
            "amount_usd": intent.amount_usd,
            "side": intent.side,
        });
        let v = self.post_json("/swap/quote", body).await?;
        parse::swap_quote(&v, intent)
            .ok_or_else(|| TwakError::Parse("malformed /swap/quote response".into()))
    }

    async fn execute_swap(&self, approved: &ApprovedOrder) -> Result<TxReceipt, TwakError> {
        if !self.autonomous {
            // Non-autonomous clients require the order to already carry an
            // approval; the presence of `ApprovedOrder` is that gate. We still
            // surface the distinction so callers can route signing flows.
            tracing::debug!(order = %approved.id, "REST swap executing under approval gate");
        }
        let body = json!({
            "order_id": approved.id,
            "from_symbol": approved.intent.from_symbol,
            "to_symbol": approved.intent.to_symbol,
            "amount_usd": approved.approved_amount_usd,
            "autonomous": self.autonomous,
        });
        let v = self.post_json("/swap/execute", body).await?;
        Ok(parse::tx_receipt(&v))
    }

    async fn register_competition(&self) -> Result<TxReceipt, TwakError> {
        let v = self.post_json("/compete/register", json!({})).await?;
        Ok(parse::tx_receipt(&v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_trims_trailing_slash() {
        let c = TwakRestClient::new("https://twak.example/", true);
        assert_eq!(c.url("/wallet"), "https://twak.example/wallet");
        assert!(c.is_autonomous());
    }

    #[test]
    fn url_handles_missing_leading_slash() {
        let c = TwakRestClient::new("https://twak.example", false);
        assert_eq!(c.url("wallet"), "https://twak.example/wallet");
        assert!(!c.is_autonomous());
    }
}
