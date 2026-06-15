//! CoinMarketCap data access.
//!
//! This crate owns *all* CMC communication and nothing else. It exposes a
//! single trait, [`CmcDataSource`], plus a REST client and a deterministic
//! mock used in paper/backtest mode. Downstream crates depend on the trait,
//! never on the wire format.

pub mod client;
pub mod endpoints;
pub mod error;
pub mod mcp;
pub mod mock;
pub mod models;
pub mod rate_limit;
pub mod rest;
pub mod retry;
pub mod x402;

pub use client::CmcRestClient;
pub use error::CmcError;
pub use mcp::CmcMcpClient;
pub use mock::MockCmcClient;
pub use models::{
    Candle, CmcQuote, DexLiquidity, FearGreedSnapshot, GlobalMarket, Interval, TokenSecurity,
    TrendingToken,
};

use async_trait::async_trait;
use common::Asset;

/// The contract every market-data provider implements.
#[async_trait]
pub trait CmcDataSource: Send + Sync {
    async fn latest_quotes(&self, assets: &[Asset]) -> Result<Vec<CmcQuote>, CmcError>;
    async fn ohlcv(&self, asset: &Asset, interval: Interval) -> Result<Vec<Candle>, CmcError>;
    async fn fear_greed(&self) -> Result<FearGreedSnapshot, CmcError>;
    async fn dex_liquidity(&self, asset: &Asset) -> Result<DexLiquidity, CmcError>;
    async fn token_security(&self, asset: &Asset) -> Result<TokenSecurity, CmcError>;
    async fn trending(&self) -> Result<Vec<TrendingToken>, CmcError>;
    async fn global_market(&self) -> Result<GlobalMarket, CmcError>;
}

/// Selects which wire implementation backs a [`CmcDataSource`].
///
/// `Mock` is the default so paper/backtest mode runs offline; `Rest` talks to
/// the CMC Pro REST API; `Mcp` talks JSON-RPC 2.0 to a CMC AI Agent Hub MCP
/// endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CmcTransport {
    #[default]
    Mock,
    Rest,
    Mcp,
}

/// Build a boxed [`CmcDataSource`] for the requested transport.
///
/// - [`CmcTransport::Mock`] ignores all parameters and returns the offline mock.
/// - [`CmcTransport::Rest`] builds a [`CmcRestClient`] (requires `api_key`).
/// - [`CmcTransport::Mcp`] builds a [`CmcMcpClient`] (requires `mcp_url`).
pub fn source_from(
    transport: CmcTransport,
    api_key: impl Into<String>,
    mcp_url: impl Into<String>,
    timeout_ms: u64,
) -> Result<Box<dyn CmcDataSource>, CmcError> {
    match transport {
        CmcTransport::Mock => Ok(Box::new(MockCmcClient::new())),
        CmcTransport::Rest => Ok(Box::new(CmcRestClient::new(api_key, timeout_ms)?)),
        CmcTransport::Mcp => Ok(Box::new(CmcMcpClient::new(mcp_url, timeout_ms)?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_from_mock_is_ok() {
        let source = source_from(CmcTransport::Mock, "", "", 1000);
        assert!(source.is_ok());
    }

    #[test]
    fn transport_defaults_to_mock() {
        assert_eq!(CmcTransport::default(), CmcTransport::Mock);
    }
}
