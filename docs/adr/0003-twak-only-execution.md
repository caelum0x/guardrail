# ADR 0003 — All execution flows through TWAK (self-custody)

- Status: Accepted
- Date: 2026-06-13

## Context

Track 1 requires self-custody and execution via the Trust Wallet Agent Kit
(TWAK). The agent must never hold raw private keys in process.

## Decision

`twak-client` is the only component that can produce a signature or an on-chain
trade, behind a single `TwakExecutor` trait (`wallet_address`, `portfolio`,
`quote_swap`, `execute_swap`, `register_competition`). The execution flow is
strictly: `OrderIntent → pre-trade risk → TWAK quote → final risk → TWAK execute
→ receipt → portfolio reconcile → event`. `policy.execution_layer = "twak_only"`
and `bypass_twak` is a forbidden action.

For x402-gated data (e.g. CMC premium), the client builds the payment
authorization but **TWAK signs it** (`twak-client::x402::sign_authorization`) —
keys stay with the wallet.

## Consequences

- Paper mode uses `MockTwakClient` (deterministic, no keys/network) so the full
  pipeline runs offline and reproducibly.
- Swapping the live transport (MCP/REST/CLI) is a single trait implementation;
  no caller changes.
- The dashboard, API, and Python analytics are physically incapable of
  executing — they don't depend on `twak-client`.
