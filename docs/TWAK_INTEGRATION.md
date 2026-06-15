# TWAK Integration

The `twak-client` crate is the thin Trust Wallet Agent Kit boundary and the
**sole execution layer**. It is the only component in the workspace that can
produce a signature or an on-chain trade. It depends on `risk-engine` for the
`ApprovedOrder` type, which structurally guarantees nothing is executed without
an approval (see [ADR 0003](adr/0003-twak-only-execution.md) and
[SELF_CUSTODY.md](SELF_CUSTODY.md)).

## `TwakExecutor` trait

The contract every executor implements (`crates/twak-client/src/lib.rs`):

```rust
#[async_trait]
pub trait TwakExecutor: Send + Sync {
    async fn wallet_address(&self) -> Result<Address, TwakError>;
    async fn portfolio(&self) -> Result<TwakPortfolio, TwakError>;
    async fn quote_swap(&self, intent: &OrderIntent) -> Result<SwapQuote, TwakError>;
    async fn execute_swap(&self, approved: &ApprovedOrder) -> Result<TxReceipt, TwakError>;
    async fn register_competition(&self) -> Result<TxReceipt, TwakError>;
}
```

Note the asymmetry that enforces the guardrail at the type level: `quote_swap`
takes a bare `OrderIntent` (quoting is authority-free), but `execute_swap`
requires an `ApprovedOrder` — a type only the risk engine can mint. You cannot
call `execute_swap` without having passed the risk gate.

Types: `SwapQuote { route_id, expected_out_symbol, expected_out_amount, summary }`
where `summary: QuoteSummary` carries expected-out USD, price impact, slippage,
and liquidity; `TxReceipt { tx_hash, status, block_number }`; `TwakPortfolio` /
`TwakBalance`.

## Transports

`twak-client` exposes the executor behind one trait with swappable transports,
selected by `twak.mode` in config (`"mock" | "rest" | "mcp" | "cli"` —
`crates/common/src/config.rs::TwakCfg`):

- **Mock** (`mock.rs`, `MockTwakClient`) — the default. Deterministic, key-free,
  network-free; the only executor wired into `agent-runtime` today so paper mode
  runs fully offline.
- **REST** (`rest.rs`, `TwakRestConfig { url }`) — HTTP transport to a TWAK REST
  endpoint that holds the wallet and signs.
- **MCP** (`mcp.rs`, `TwakMcpConfig { url }`) — Model Context Protocol transport
  to the Trust Wallet agent surface.
- **CLI** (`cli.rs`, `TWAK_CLI = "twak"`) — shells out to the `twak` CLI for
  signing/execution.

Swapping the live transport is a single trait implementation; no caller changes,
because the runtime depends only on `TwakExecutor`.

## Signing, autonomous mode, and x402

- **Signing stays with TWAK.** The engine builds `OrderIntent`s and (after
  approval) hands an `ApprovedOrder` to the executor. The wallet — never the
  engine — signs. No raw private key is held in process (forbidden action
  `custodial_signing`, `crates/risk-engine` policy).
- **Autonomous execution + guardrails.** `agent-runtime` runs the loop
  unattended (`apps/guardrail-agent`), but every order passes the risk engine
  twice (pre-trade and final). A kill switch (`kill_switch_drawdown_pct`) halts
  trading and stays engaged; a soft drawdown limit makes the book reduce-only.
- **x402.** `twak-client::x402::sign_authorization` is the self-custody signing
  entry point: when a data provider returns HTTP 402, the client builds the
  authorization payload and **TWAK signs it** (keys stay with the wallet). The
  default in-process signer is a deterministic mock; production routes signing to
  the TWAK MCP/REST signer. The CMC side of the same flow lives in
  `crates/cmc-client/src/x402.rs` (see [CMC_INTEGRATION.md](CMC_INTEGRATION.md)).

## quote → risk → execute → reconcile loop

The risk-gated sequence, driven per order by `agent-runtime`
(`crates/agent-runtime/src/runtime.rs::process_order`):

1. `RiskEngine::pre_trade(intent, ctx)` — pre-trade gate (no quote yet); reject
   early on allowlist, drawdown, reserve, position, or security-flag violations.
2. `quote_swap(intent)` → `SwapQuote` (mandatory before any swap:
   `policy.require_quote_before_swap`, `risk::TOKEN_RISK_CHECK_REQUIRED`,
   `approvals::approvals_required()`).
3. `RiskEngine::approve(intent, ctx, &quote.summary)` → `ApprovedOrder` (or a
   rejecting `RiskDecision`). The final check re-runs all checks plus the
   quote-aware slippage and liquidity checks; it may also clip the size.
4. `execute_swap(&approved)` → `TxReceipt`.
5. Reconcile: `apply_fill` updates `PortfolioState`; an `AgentEvent` is appended
   to the SQLite + in-memory event log at every stage (`OrderProposed`,
   `TwakQuoteReceived`, `RiskApproved`/`RiskClipped`, `TwakSwapSubmitted`,
   `TxConfirmed`).

## Mock (`MockTwakClient`)

`mock.rs` is a deterministic executor for paper trading and tests — and the
default so the pipeline runs offline. It models an AMM: price impact grows with
the fraction of a notional pool (`$3M` default) consumed, and slippage ≈ half the
impact plus a fixed 0.05% venue spread. `quote_swap` returns an internally
consistent `SwapQuote`; `execute_swap` logs the swap and returns a `confirmed`
`TxReceipt` with a synthetic hash and block number; `register_competition`
returns a confirmed registration receipt. Construct with `MockTwakClient::new()`
or `with_address(...)`.

## Competition registration

Track 1 requires on-chain registration of the competing agent wallet before
trading. `agent-runtime` calls `executor.register_competition()` when
`twak.competition_register_enabled` is set (`runtime.rs`), recording the tx as a
`TxConfirmed` event. The competition contract is pinned in
`crates/common/src/constants.rs`:

```
COMPETITION_CONTRACT = 0x212c61b9b72c95d95bf29cf032f5e5635629aed5
```

surfaced by `guardrail-cli register` and overridable via the
`COMPETITION_CONTRACT` env (`twak-client::competition::COMPETITION_CONTRACT_ENV`).
The live registration runs through TWAK:

```
twak compete register     # (or scripts/register_agent.sh)
```

## "Best Use of TWAK" scoring map

| Criterion (weight) | Where it lives |
|---|---|
| **Integration depth (30)** | `TwakExecutor` is the only execution path; four transports (Mock/REST/MCP/CLI); full quote → risk → execute → reconcile loop wired in `agent-runtime`; modules for wallet, portfolio, quote, swap, tx history, token risk, competition, x402. |
| **Self-custody integrity (25)** | `execute_swap` requires an `ApprovedOrder` (type-enforced gate); signing stays in TWAK; `custodial_signing`/`bypass_twak` are forbidden actions; dashboard/API/analytics don't depend on `twak-client`. See [SELF_CUSTODY.md](SELF_CUSTODY.md). |
| **Autonomous execution + guardrails (20)** | Unattended loop (`apps/guardrail-agent`); dual risk gate; kill switch + reduce-only drawdown throttle; daily-trade heartbeat. |
| **x402 (10)** | `x402::sign_authorization` (self-custody signing) paired with the CMC `pay_and_retry` 402 loop. |
| **Originality (10)** | Approval-as-type (`ApprovedOrder`) makes "execute without risk approval" unrepresentable; deterministic mock transport for reproducible offline demos. |
| **Demo (5)** | `guardrail-cli quote`, `register`; the agent loop prints the regime, orders, and confirmed tx hashes per cycle. |
