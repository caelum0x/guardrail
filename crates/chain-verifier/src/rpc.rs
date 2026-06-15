//! Minimal read-only Ethereum-JSON-RPC client for BSC.
//!
//! Only the three read methods the verifier needs are implemented. There is no
//! signing, no `eth_sendTransaction`, no account management — by construction
//! this client cannot move funds or change chain state.

use serde_json::{json, Value};
use std::time::Duration;

const DEFAULT_TIMEOUT_MS: u64 = 8_000;

/// Errors raised by the JSON-RPC client. All are recoverable by the caller into
/// a failed verification check rather than a process abort.
#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    /// Transport-level failure (connect/timeout/TLS/etc.).
    #[error("rpc transport error: {0}")]
    Http(#[from] reqwest::Error),
    /// The node returned a JSON-RPC `error` object.
    #[error("rpc error: {0}")]
    Rpc(String),
    /// The response was missing `result` or had an unexpected shape.
    #[error("unexpected rpc response shape")]
    Shape,
}

/// A decoded subset of an `eth_getTransactionReceipt` result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    /// `1` for success, `0` for revert (decoded from the hex `status` field).
    pub status: Option<u64>,
    /// Recipient address (the contract, for a registration call).
    pub to: Option<String>,
    /// Sender address (the wallet or a sponsoring relayer).
    pub from: Option<String>,
    /// Block the transaction was included in.
    pub block_number: Option<u64>,
}

/// A thin POST-based JSON-RPC client bound to a single endpoint URL.
pub struct BscRpcClient {
    http: reqwest::Client,
    url: String,
}

impl BscRpcClient {
    /// Builds a client with a bounded request timeout.
    pub fn new(url: impl Into<String>) -> Result<Self, RpcError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(DEFAULT_TIMEOUT_MS))
            .build()?;
        Ok(Self {
            http,
            url: url.into(),
        })
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        let resp = self.http.post(&self.url).json(&body).send().await?;
        let value: Value = resp.json().await?;
        if let Some(err) = value.get("error") {
            return Err(RpcError::Rpc(err.to_string()));
        }
        value.get("result").cloned().ok_or(RpcError::Shape)
    }

    /// `eth_chainId` decoded to a `u64` (BSC mainnet is `56`).
    pub async fn chain_id(&self) -> Result<u64, RpcError> {
        let result = self.call("eth_chainId", json!([])).await?;
        result
            .as_str()
            .and_then(parse_hex_u64)
            .ok_or(RpcError::Shape)
    }

    /// `eth_getCode(address, "latest")` returned as the raw `0x…` hex string.
    pub async fn get_code(&self, address: &str) -> Result<String, RpcError> {
        let result = self.call("eth_getCode", json!([address, "latest"])).await?;
        result
            .as_str()
            .map(str::to_string)
            .ok_or(RpcError::Shape)
    }

    /// `eth_getTransactionReceipt(tx)`. Returns `Ok(None)` when the node knows
    /// of no such transaction (a `null` result), which is distinct from an error.
    pub async fn get_transaction_receipt(&self, tx: &str) -> Result<Option<Receipt>, RpcError> {
        let result = self
            .call("eth_getTransactionReceipt", json!([tx]))
            .await?;
        if result.is_null() {
            return Ok(None);
        }
        Ok(Some(Receipt {
            status: field_hex_u64(&result, "status"),
            to: field_str(&result, "to"),
            from: field_str(&result, "from"),
            block_number: field_hex_u64(&result, "blockNumber"),
        }))
    }
}

fn field_str(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn field_hex_u64(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_str).and_then(parse_hex_u64)
}

/// Parse a `0x`-prefixed (or bare) hex string into a `u64`. Returns `None` on
/// any non-hex input or overflow.
pub fn parse_hex_u64(s: &str) -> Option<u64> {
    let body = s.strip_prefix("0x").unwrap_or(s);
    if body.is_empty() {
        return None;
    }
    u64::from_str_radix(body, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_chain_id() {
        assert_eq!(parse_hex_u64("0x38"), Some(56));
        assert_eq!(parse_hex_u64("0x1"), Some(1));
        assert_eq!(parse_hex_u64("38"), Some(56));
    }

    #[test]
    fn rejects_non_hex() {
        assert_eq!(parse_hex_u64("0x"), None);
        assert_eq!(parse_hex_u64(""), None);
        assert_eq!(parse_hex_u64("0xzz"), None);
    }

    #[test]
    fn decodes_receipt_fields() {
        let raw = json!({
            "status": "0x1",
            "to": "0xABCD",
            "from": "0xWALLET",
            "blockNumber": "0x10"
        });
        assert_eq!(field_hex_u64(&raw, "status"), Some(1));
        assert_eq!(field_hex_u64(&raw, "blockNumber"), Some(16));
        assert_eq!(field_str(&raw, "to").as_deref(), Some("0xABCD"));
    }
}
