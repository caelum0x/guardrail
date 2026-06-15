use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxReceipt {
    pub tx_hash: String,
    pub status: String,
    pub block_number: Option<u64>,
}

impl TxReceipt {
    pub fn mock(tx_hash: impl Into<String>) -> Self {
        Self {
            tx_hash: tx_hash.into(),
            status: "submitted".into(),
            block_number: None,
        }
    }
}
