# Execution

`execution` turns an approved order into a TWAK swap. It never decides *whether*
to trade — that is the risk engine's job — it only orchestrates the ordered
sequence and reconciles the result.

## Pipeline: OrderIntent → execution

`execution::quote_then_approve` (and the equivalent inline path in
`agent-runtime::process_order`):

1. **OrderIntent** — a strategy proposal (`side`, `from_symbol`, `to_symbol`,
   `amount_usd`, `reason`). Intent only; carries no authority.
2. **Pre-trade risk** — `RiskEngine::pre_trade(intent, ctx)`.
   - `Rejected` → stop, return `ExecutionError::RiskRejected`.
   - `Clipped` → carry the reduced `new_amount_usd` forward into the quote.
   - `Approved` → proceed unchanged.
3. **TWAK quote** — `TwakExecutor::quote_swap(intent)` returns a `SwapQuote`
   whose `summary: QuoteSummary` (expected out, price impact, slippage,
   liquidity) is the only quote data risk sees.
4. **Final risk** — `RiskEngine::approve(intent, ctx, &quote.summary)` re-runs
   the pre-trade checks plus the quote-aware slippage and liquidity checks. On
   pass it returns an `ApprovedOrder { id, intent, approved_amount_usd,
   decision }`; otherwise it returns the rejecting `RiskDecision`.
5. **Execute** — `TwakExecutor::execute_swap(&approved)` submits the swap and
   returns a `TxReceipt { tx_hash, status, block_number }`.
6. **Reconcile** — the fill (notional, prices, slippage + gas fee) is applied to
   the portfolio via `portfolio::trade_accounting::apply_fill`, updating NAV and
   holdings.

Every stage emits an `AgentEvent` (`OrderProposed`, `TwakQuoteReceived`,
`RiskApproved` / `RiskClipped` / `RiskRejected`, `TwakSwapSubmitted`,
`TxConfirmed`, `PortfolioReconciled`) to the event store.

## Guarantees

- Risk is consulted **twice** — before and after the quote — so a quote that
  reveals excessive slippage or vanished liquidity is rejected even if the
  pre-trade check passed.
- A quote is **mandatory** before any swap (`require_quote_before_swap`,
  `pre_trade::QUOTE_BEFORE_SWAP`).
- The only route is `twak` (`router::route_name`); there is no alternate
  execution path.
- `reconciliation::reconciliation_required()` is always true — the book is
  reconciled after every fill.
