# TWAK Self-Custody Demo

This document is the written form of
[`scripts/self_custody_demo.sh`](../scripts/self_custody_demo.sh) — a narrated,
**offline** walkthrough of how Guardrail keeps custody with the user's wallet
through the Trust Wallet Agent Kit (TWAK). It maps each step of the flow to the
TWAK prize criteria and points at the real source of truth in this repo.

> Run it: `./scripts/self_custody_demo.sh` (no network, no keys required).

For the deeper design rationale, see [`docs/SELF_CUSTODY.md`](./SELF_CUSTODY.md)
and ADR 0003 (all execution flows through TWAK).

## The invariant

> Nothing in the workspace except `twak-client` can produce a signature or an
> on-chain trade — and even `twak-client` only executes an order the risk engine
> has already approved.

The agent **proposes**; it can never sign or move funds. This is enforced by the
type system and the dependency graph, not by convention, so it cannot be silently
regressed without breaking compilation.

## The flow

```
agent proposes  ->  risk engine gates  ->  TWAK signs (user-held keys)  ->  execution + reconcile
```

### Step 1 — Agent PROPOSES an intent (no keys, no signing)

The strategy engine builds an `OrderIntent` — plain data describing *what* should
happen. Quoting is authority-free: `quote_swap(intent)` is read-only and touches
no keys.

- Source: [`crates/agent-runtime/src/runtime.rs`](../crates/agent-runtime/src/runtime.rs),
  [`crates/twak-client/src/quote.rs`](../crates/twak-client/src/quote.rs)
- Routes: `/signals`, `/quotes`

### Step 2 — Risk engine GATES the proposal (the only door to TWAK)

`RiskEngine` runs a pre-trade check, then `approve(quote)` either mints an
`ApprovedOrder` or rejects/clips. `ApprovedOrder` has **no public constructor**,
so "execute without a risk approval" is a *compile error*, not a runtime check.
`require_quote_before_swap = true` forces a fresh quote first.

- Source: [`crates/twak-client/src/risk.rs`](../crates/twak-client/src/risk.rs)
  (`TOKEN_RISK_CHECK_REQUIRED = true`),
  [`configs/risk_policy.production.json`](../configs/risk_policy.production.json),
  [`configs/signing_policy.example.json`](../configs/signing_policy.example.json)
- Routes: `/risk`, `/policy`

The example signing policy declares the authorization envelope the user's wallet
enforces:

| Control | Example value |
|---------|---------------|
| Custody model | `self_custody` — signer `twak`, `agent_can_sign: false`, `agent_holds_keys: false` |
| Per-tx cap | `500` USD, `max_position_pct: 18`, `max_new_position_pct: 12` |
| Daily cap | `2000` USD over at most `12` transactions, `max_drawdown_pct: 7` |
| Slippage bound | `<= 0.8%`, `require_quote_before_swap: true` |
| Allowed actions | `quote_swap`, `execute_swap`, `register_competition`, `x402_sign_authorization` |
| Forbidden actions | `custodial_signing`, `bypass_twak`, `key_export`, `delegate_signing`, `launch_token`, `borrow_without_policy`, `trade_non_eligible_assets` |
| Allowed contracts | competition contract + payment token only |

### Step 3 — TWAK SIGNS with the user-held keys

Only TWAK signs and broadcasts. The sole funds-moving method,
`execute_swap(approved: &ApprovedOrder)`, admits an engine-minted `ApprovedOrder`
and nothing else. For x402-gated data, the agent builds the payment payload but
**TWAK signs it** — keys never leave the wallet.

- Source: [`crates/twak-client/src/x402.rs`](../crates/twak-client/src/x402.rs)
  (`sign_authorization`),
  [`crates/twak-client/src/swap.rs`](../crates/twak-client/src/swap.rs),
  [`crates/twak-client/src/mock.rs`](../crates/twak-client/src/mock.rs) (offline default)
- Route: `/signing-policy`

The demo prints a deterministic signer demonstration that mirrors the real
formula `sha256(signer || 0x00 || authorization)` from `x402.rs`. It uses **no
real key material** — it illustrates that signing is a TWAK concern and the agent
process never holds a private key.

### Step 4 — EXECUTION + reconcile (auditable, read-only surfaces)

TWAK returns a `TxReceipt`; the runtime reconciles the portfolio and logs events.
The API is read-only and the dashboard does not depend on `twak-client`, so
neither has any path to signing or execution — least privilege enforced by the
dependency graph.

- Source: [`data/run_report.json`](../data/run_report.json)
- Routes: `/proof`, `/readiness`, `/compete`

The resulting proof can be verified independently and offline:

```bash
./scripts/verify_proof.sh
```

See [`docs/PROOF_VERIFICATION.md`](./PROOF_VERIFICATION.md).

## Mapping to TWAK prize criteria

| Criterion | How Guardrail satisfies it |
|-----------|----------------------------|
| Keys stay with the user | No raw key is loaded by the engine; `custodial_signing` and `key_export` are forbidden actions. |
| Signing authority stays with the user's wallet | All signing happens inside TWAK (`x402::sign_authorization`, transport signers); the agent only proposes. |
| Agent cannot act autonomously with funds | `execute_swap` requires an unforgeable `ApprovedOrder` minted only by the risk engine. |
| Bounded authorizations | Per-tx and daily caps, slippage bound, allow/deny lists declared in `configs/signing_policy.example.json`. |
| No co-signing / delegation | `delegate_signing` and `bypass_twak` are forbidden; TWAK is the sole signer and execution venue. |
| Same guarantees offline and live | `MockTwakClient` exercises the identical `quote -> approve -> execute` path with no keys/network; REST/MCP/CLI transports keep the same shape. |

## Safety properties demonstrated

- [x] Keys remain with the user's wallet; the agent holds none.
- [x] Signing authority remains with TWAK; the agent only proposes.
- [x] Risk engine is the only gate; `ApprovedOrder` is unforgeable (type-enforced).
- [x] Per-tx + daily caps, slippage bound, allow/forbid lists are policy-declared.
- [x] `custodial_signing` / `bypass_twak` / `key_export` are forbidden actions.
- [x] Same shape offline (`MockTwakClient`) and live (REST/MCP/CLI transports).
