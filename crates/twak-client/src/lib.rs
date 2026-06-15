//! Thin Trust Wallet Agent Kit boundary.

pub mod approvals;
pub mod cli;
pub mod competition;
pub mod error;
pub mod mcp;
pub mod mock;
pub mod parse;
pub mod portfolio;
pub mod quote;
pub mod rest;
pub mod risk;
pub mod swap;
pub mod tx_history;
pub mod wallet;
pub mod x402;

pub use cli::TwakCliClient;
pub use error::TwakError;
pub use mcp::TwakMcpClient;
pub use mock::MockTwakClient;
pub use portfolio::{TwakBalance, TwakPortfolio};
pub use quote::SwapQuote;
pub use rest::TwakRestClient;
pub use swap::TxReceipt;
pub use x402::{sign_authorization, SignedAuthorization};

use async_trait::async_trait;
use common::{Address, OrderIntent};
use risk_engine::ApprovedOrder;

#[async_trait]
pub trait TwakExecutor: Send + Sync {
    async fn wallet_address(&self) -> Result<Address, TwakError>;
    async fn portfolio(&self) -> Result<TwakPortfolio, TwakError>;
    async fn quote_swap(&self, intent: &OrderIntent) -> Result<SwapQuote, TwakError>;
    async fn execute_swap(&self, approved: &ApprovedOrder) -> Result<TxReceipt, TwakError>;
    async fn register_competition(&self) -> Result<TxReceipt, TwakError>;
}

/// Which TWAK execution surface to drive. `Mock` is the offline default that
/// keeps paper mode running without network or keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TwakTransport {
    /// Deterministic in-process executor (default, offline).
    #[default]
    Mock,
    /// HTTP REST surface.
    Rest,
    /// JSON-RPC 2.0 over HTTP (MCP).
    Mcp,
    /// Shell out to the `twak` CLI (registration / wallet only).
    Cli,
}

/// Build a boxed [`TwakExecutor`] for the requested transport.
///
/// `base_url` is used by the REST and MCP surfaces (ignored by Mock/Cli) and
/// `autonomous` gates self-submitted swaps on the REST surface. Anything other
/// than a fully-specified network transport falls back to the offline
/// [`MockTwakClient`] so paper mode always has a working executor.
pub fn executor_from(
    transport: TwakTransport,
    base_url: Option<&str>,
    autonomous: bool,
) -> Box<dyn TwakExecutor> {
    match transport {
        TwakTransport::Rest => match base_url {
            Some(url) => Box::new(TwakRestClient::new(url, autonomous)),
            None => Box::new(MockTwakClient::new()),
        },
        TwakTransport::Mcp => match base_url {
            Some(url) => Box::new(TwakMcpClient::new(url)),
            None => Box::new(MockTwakClient::new()),
        },
        TwakTransport::Cli => Box::new(TwakCliClient::default()),
        TwakTransport::Mock => Box::new(MockTwakClient::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn factory_returns_mock_for_mock_transport() {
        let exec = executor_from(TwakTransport::Mock, None, false);
        let addr = exec.wallet_address().await.expect("mock address");
        assert!(addr.0.starts_with("0x"));
    }

    #[tokio::test]
    async fn factory_falls_back_to_mock_without_url() {
        // REST/MCP without a base_url degrade to the offline mock.
        let rest = executor_from(TwakTransport::Rest, None, true);
        assert!(rest.portfolio().await.is_ok());
        let mcp = executor_from(TwakTransport::Mcp, None, false);
        assert!(mcp.portfolio().await.is_ok());
    }

    #[test]
    fn default_transport_is_mock() {
        assert_eq!(TwakTransport::default(), TwakTransport::Mock);
    }
}
