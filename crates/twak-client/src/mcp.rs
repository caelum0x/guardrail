//! MCP (JSON-RPC 2.0 over HTTP) execution surface for the Trust Wallet Agent Kit.
//!
//! `TwakMcpClient` invokes TWAK tools by name over JSON-RPC. Each `TwakExecutor`
//! method maps to an MCP method ("wallet_address", "get_portfolio", "quote_swap",
//! "execute_swap", "competition_register") and the JSON-RPC `result` is parsed
//! defensively through [`crate::parse`]. When a swap call returns a 402-style
//! payment requirement the client signs an x402 authorization (see
//! [`crate::x402`]) and retries with the payment in params.

use crate::error::TwakError;
use crate::parse;
use crate::portfolio::TwakPortfolio;
use crate::quote::SwapQuote;
use crate::swap::TxReceipt;
use crate::{x402, TwakExecutor};
use async_trait::async_trait;
use common::{Address, OrderIntent};
use risk_engine::ApprovedOrder;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

/// Legacy single-URL config; prefer [`TwakMcpClient::new`].
#[derive(Debug, Clone)]
pub struct TwakMcpConfig {
    pub url: String,
}

/// JSON-RPC 2.0 client for the TWAK MCP execution surface.
pub struct TwakMcpClient {
    url: String,
    http: reqwest::Client,
    id: AtomicU64,
}

impl Clone for TwakMcpClient {
    fn clone(&self) -> Self {
        TwakMcpClient {
            url: self.url.clone(),
            http: self.http.clone(),
            id: AtomicU64::new(self.id.load(Ordering::Relaxed)),
        }
    }
}

impl TwakMcpClient {
    /// Build an MCP client targeting the JSON-RPC `url`.
    pub fn new(url: impl Into<String>) -> Self {
        TwakMcpClient {
            url: url.into(),
            http: reqwest::Client::new(),
            id: AtomicU64::new(1),
        }
    }

    /// Invoke an MCP method over JSON-RPC 2.0 and return the `result` value.
    ///
    /// JSON-RPC `error` objects are mapped to [`TwakError::Rejected`]; transport
    /// and decode failures map to [`TwakError::Transport`] / [`TwakError::Parse`].
    pub async fn call(&self, method: &str, params: Value) -> Result<Value, TwakError> {
        let id = self.id.fetch_add(1, Ordering::Relaxed);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let resp = self
            .http
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| TwakError::Transport(e.to_string()))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| TwakError::Transport(e.to_string()))?;

        if !status.is_success() {
            return Err(TwakError::Rejected(format!(
                "TWAK MCP {status}: {}",
                text.chars().take(200).collect::<String>()
            )));
        }

        let body: Value =
            serde_json::from_str(&text).map_err(|e| TwakError::Parse(e.to_string()))?;

        if let Some(err) = body.get("error") {
            let message = err
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown JSON-RPC error");
            return Err(TwakError::Rejected(format!("TWAK MCP: {message}")));
        }

        body.get("result")
            .cloned()
            .ok_or_else(|| TwakError::Parse("JSON-RPC response missing result".into()))
    }

    /// Call a swap method, signing and retrying once if the venue reports an
    /// x402 payment requirement inside the JSON-RPC result.
    async fn call_swap(&self, method: &str, mut params: Value) -> Result<Value, TwakError> {
        let result = self.call(method, params.clone()).await?;
        if let Some(terms) = payment_required(&result) {
            let signed = x402::sign_authorization(&terms, &self.url);
            let payment =
                serde_json::to_value(&signed).map_err(|e| TwakError::Parse(e.to_string()))?;
            if let Some(obj) = params.as_object_mut() {
                obj.insert("x402_payment".to_string(), payment);
            }
            return self.call(method, params).await;
        }
        Ok(result)
    }
}

/// Detect an x402 payment challenge embedded in a JSON-RPC result.
fn payment_required(result: &Value) -> Option<String> {
    let needs = result
        .get("payment_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || result.get("status").and_then(Value::as_str) == Some("payment_required");
    if !needs {
        return None;
    }
    result
        .get("accepts")
        .or_else(|| result.get("payment_terms"))
        .map(|t| t.to_string())
        .or_else(|| Some(result.to_string()))
}

#[async_trait]
impl TwakExecutor for TwakMcpClient {
    async fn wallet_address(&self) -> Result<Address, TwakError> {
        let v = self.call("wallet_address", json!({})).await?;
        let addr = parse::wallet_address(&v)
            .ok_or_else(|| TwakError::Parse("no wallet address in MCP result".into()))?;
        Ok(Address::new(addr))
    }

    async fn portfolio(&self) -> Result<TwakPortfolio, TwakError> {
        let v = self.call("get_portfolio", json!({})).await?;
        Ok(parse::portfolio(&v))
    }

    async fn quote_swap(&self, intent: &OrderIntent) -> Result<SwapQuote, TwakError> {
        let params = json!({
            "from_symbol": intent.from_symbol,
            "to_symbol": intent.to_symbol,
            "amount_usd": intent.amount_usd,
            "side": intent.side,
        });
        let v = self.call_swap("quote_swap", params).await?;
        parse::swap_quote(&v, intent)
            .ok_or_else(|| TwakError::Parse("malformed quote_swap result".into()))
    }

    async fn execute_swap(&self, approved: &ApprovedOrder) -> Result<TxReceipt, TwakError> {
        let params = json!({
            "order_id": approved.id,
            "from_symbol": approved.intent.from_symbol,
            "to_symbol": approved.intent.to_symbol,
            "amount_usd": approved.approved_amount_usd,
        });
        let v = self.call_swap("execute_swap", params).await?;
        Ok(parse::tx_receipt(&v))
    }

    async fn register_competition(&self) -> Result<TxReceipt, TwakError> {
        let v = self.call("competition_register", json!({})).await?;
        Ok(parse::tx_receipt(&v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_required_detects_flag() {
        assert!(payment_required(&json!({"payment_required": true})).is_some());
        assert!(payment_required(&json!({"status": "payment_required"})).is_some());
        assert!(payment_required(&json!({"status": "ok"})).is_none());
    }

    #[test]
    fn client_is_constructible() {
        let c = TwakMcpClient::new("https://twak.example/mcp");
        assert_eq!(c.url, "https://twak.example/mcp");
    }
}
