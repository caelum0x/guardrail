//! Applies fills to the portfolio book. A fill moves value from one symbol to
//! another at an execution price, updating quantities, average cost, and
//! realized PnL.

use crate::holding::Holding;
use crate::portfolio_state::PortfolioState;
use common::Decimal;
use serde::{Deserialize, Serialize};

/// A confirmed fill, expressed in symbol terms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub from_symbol: String,
    pub to_symbol: String,
    /// USD notional traded (gross).
    pub notional_usd: Decimal,
    /// Price of the `to` asset in USD at execution.
    pub to_price_usd: Decimal,
    /// Price of the `from` asset in USD at execution.
    pub from_price_usd: Decimal,
    /// Fees paid, in USD.
    pub fee_usd: Decimal,
}

/// Apply a fill to the portfolio, mutating it in place.
pub fn apply_fill(state: &mut PortfolioState, fill: &Fill) {
    // Reduce the `from` holding by the notional value sold.
    if fill.from_price_usd > Decimal::ZERO {
        let from_qty = fill.notional_usd / fill.from_price_usd;
        reduce(state, &fill.from_symbol, from_qty);
    }

    // Increase the `to` holding by the net notional received.
    let net_usd = (fill.notional_usd - fill.fee_usd).max(Decimal::ZERO);
    if fill.to_price_usd > Decimal::ZERO {
        let to_qty = net_usd / fill.to_price_usd;
        increase(state, &fill.to_symbol, to_qty, fill.to_price_usd);
    }

    // Fees are a realized cost.
    state.realized_pnl_usd -= fill.fee_usd;
}

fn increase(state: &mut PortfolioState, symbol: &str, qty: Decimal, price_usd: Decimal) {
    if let Some(h) = state.get_mut(symbol) {
        let new_qty = h.quantity + qty;
        if new_qty > Decimal::ZERO {
            // Volume-weighted average cost.
            h.avg_cost_usd = (h.cost_basis_usd() + qty * price_usd) / new_qty;
        }
        h.quantity = new_qty;
        h.price_usd = price_usd;
    } else {
        state.holdings.push(Holding {
            symbol: symbol.to_string(),
            quantity: qty,
            avg_cost_usd: price_usd,
            price_usd,
        });
    }
}

fn reduce(state: &mut PortfolioState, symbol: &str, qty: Decimal) {
    // Realize PnL on the reduced quantity, then shrink the holding.
    let realized = match state.get(symbol) {
        Some(h) => qty * (h.price_usd - h.avg_cost_usd),
        None => Decimal::ZERO,
    };
    state.realized_pnl_usd += realized;
    if let Some(h) = state.get_mut(symbol) {
        h.quantity = (h.quantity - qty).max(Decimal::ZERO);
    }
    // Drop fully-closed positions, but always keep the stable reserve row.
    state
        .holdings
        .retain(|h| h.quantity > Decimal::ZERO || h.symbol == common::constants::RESERVE_SYMBOL);
}
