use common::{Decimal, QuoteSummary};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    pub route_id: String,
    pub expected_out_symbol: String,
    pub expected_out_amount: Decimal,
    pub summary: QuoteSummary,
}
