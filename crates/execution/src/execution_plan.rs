use common::OrderIntent;
use serde::{Deserialize, Serialize};
use twak_client::TxReceipt;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub orders: Vec<OrderIntent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub receipts: Vec<TxReceipt>,
}
