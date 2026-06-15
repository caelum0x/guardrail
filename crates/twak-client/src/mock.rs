//! Deterministic mock executor for paper trading and tests.
//!
//! Models slippage as a function of trade size versus a notional pool and
//! returns internally consistent quotes and receipts. No keys, no network.

use crate::error::TwakError;
use crate::portfolio::{TwakBalance, TwakPortfolio};
use crate::quote::SwapQuote;
use crate::swap::TxReceipt;
use crate::TwakExecutor;
use async_trait::async_trait;
use common::{Address, Decimal, OrderIntent, QuoteSummary};
use risk_engine::ApprovedOrder;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct MockTwakClient {
    address: String,
    pool_usd: Decimal,
    nonce: AtomicU64,
}

impl Default for MockTwakClient {
    fn default() -> Self {
        MockTwakClient {
            address: "0xA9e5C0FfEe0000000000000000000000000A1b2C3".to_string(),
            pool_usd: Decimal::new(3_000_000, 0),
            nonce: AtomicU64::new(1),
        }
    }
}

impl MockTwakClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_address(address: impl Into<String>) -> Self {
        MockTwakClient {
            address: address.into(),
            ..Default::default()
        }
    }

    fn next_hash(&self, tag: &str) -> String {
        let n = self.nonce.fetch_add(1, Ordering::Relaxed);
        format!("0x{tag}{n:0>60}")
    }
}

#[async_trait]
impl TwakExecutor for MockTwakClient {
    async fn wallet_address(&self) -> Result<Address, TwakError> {
        Ok(Address::new(self.address.clone()))
    }

    async fn portfolio(&self) -> Result<TwakPortfolio, TwakError> {
        Ok(TwakPortfolio {
            balances: vec![TwakBalance {
                symbol: "USDT".to_string(),
                amount: Decimal::new(10_000, 0),
                value_usd: Decimal::new(10_000, 0),
            }],
        })
    }

    async fn quote_swap(&self, intent: &OrderIntent) -> Result<SwapQuote, TwakError> {
        let hundred = Decimal::from(100);
        // Price impact grows with the fraction of the pool consumed.
        let impact = if self.pool_usd > Decimal::ZERO {
            (intent.amount_usd / self.pool_usd * hundred).round_dp(4)
        } else {
            Decimal::ZERO
        };
        // Slippage ~ half of impact plus a fixed venue spread (0.05%).
        let slippage = (impact / Decimal::from(2) + Decimal::new(5, 2)).round_dp(4);
        let expected_out = (intent.amount_usd * (Decimal::ONE - slippage / hundred)).round_dp(2);

        Ok(SwapQuote {
            route_id: format!("q_{}", self.nonce.fetch_add(1, Ordering::Relaxed)),
            expected_out_symbol: intent.to_symbol.clone(),
            expected_out_amount: expected_out,
            summary: QuoteSummary {
                expected_out_usd: expected_out,
                price_impact_pct: impact,
                slippage_pct: slippage,
                liquidity_usd: self.pool_usd,
            },
        })
    }

    async fn execute_swap(&self, approved: &ApprovedOrder) -> Result<TxReceipt, TwakError> {
        let n = self.nonce.load(Ordering::Relaxed);
        tracing::info!(
            from = %approved.intent.from_symbol,
            to = %approved.intent.to_symbol,
            amount_usd = %approved.approved_amount_usd,
            "mock TWAK swap submitted"
        );
        Ok(TxReceipt {
            tx_hash: self.next_hash("dead"),
            status: "confirmed".to_string(),
            block_number: Some(40_000_000 + n),
        })
    }

    async fn register_competition(&self) -> Result<TxReceipt, TwakError> {
        Ok(TxReceipt {
            tx_hash: self.next_hash("reg0"),
            status: "confirmed".to_string(),
            block_number: Some(40_000_000),
        })
    }
}
