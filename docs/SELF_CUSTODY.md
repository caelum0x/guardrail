# Self-Custody

Guardrail Alpha is fully self-custodial: the user's wallet (via the Trust Wallet
Agent Kit, TWAK) holds the keys and the sole signing authority end to end. The
trading engine only *builds intents and approvals*; it can never sign or move
funds. This document explains how that property is enforced in code and maps it
to the Track 1 self-custody penalty ladder. It elaborates on
[ADR 0003 — All execution flows through TWAK](adr/0003-twak-only-execution.md).

## The invariant

> Nothing in the workspace except `twak-client` can produce a signature or an
> on-chain trade, and even `twak-client` only executes an order the risk engine
> has already approved.

## How the keys stay with the user

1. **Keys live in the wallet, not the process.** No raw private key is loaded by
   the engine. The agent never holds key material; `custodial_signing` is a
   forbidden action in the risk policy
   (`configs/risk_policy.production.json::forbidden_actions`).

2. **The engine only builds intents.** Strategy produces `OrderIntent`s
   (`crates/agent-runtime/src/runtime.rs`). The risk engine turns an approved
   intent into an `ApprovedOrder` (`crates/risk-engine`). Neither type carries
   signing authority — they are plain data describing *what* should happen.

3. **Execution authority is a type, and only the risk engine mints it.** The
   `TwakExecutor::execute_swap` signature requires an `ApprovedOrder`
   (`crates/twak-client/src/lib.rs`):

   ```rust
   async fn execute_swap(&self, approved: &ApprovedOrder) -> Result<TxReceipt, TwakError>;
   ```

   `ApprovedOrder` is constructed only inside `risk-engine` (returned from
   `RiskEngine::approve`). There is no public constructor a caller can use to
   forge one. Therefore "execute without a risk approval" is *unrepresentable* —
   it is a compile error, not a runtime check.

4. **Quoting is authority-free; executing is not.** `quote_swap` takes a bare
   `OrderIntent` (read-only, no keys). `execute_swap` is the only method that can
   move funds, and it is gated by the `ApprovedOrder` type above.

5. **TWAK does the signing.** The actual signature and broadcast happen inside
   the TWAK transport (Mock/REST/MCP/CLI — see
   [TWAK_INTEGRATION.md](TWAK_INTEGRATION.md)). For x402-gated data, the engine
   builds the payment payload but **TWAK signs it**
   (`twak-client::x402::sign_authorization`) — keys never leave the wallet.

6. **Non-execution components physically cannot trade.** The dashboard, API, and
   Python analytics do not depend on `twak-client`, so they have no path to
   signing or execution. The dependency graph itself enforces least privilege.

7. **Paper mode preserves the same shape, offline.** `MockTwakClient` is the
   default executor: deterministic, no keys, no network. It exercises the exact
   same `quote → approve → execute` path, so the self-custody guarantees are the
   same whether running offline or against a live TWAK transport.

## The risk gate is the only door to TWAK

Per order, `agent-runtime::process_order` runs:

```
OrderIntent
  → RiskEngine::pre_trade           (reject early)
  → TwakExecutor::quote_swap        (read-only, no keys)
  → RiskEngine::approve(quote)      (mints ApprovedOrder, or rejects/clips)
  → TwakExecutor::execute_swap      (TWAK signs; only an ApprovedOrder admitted)
  → portfolio reconcile + event log
```

Policy reinforces this: `execution_layer = "twak_only"` and `bypass_twak` is a
forbidden action (`configs/risk_policy.*.json`). The risk engine is the only
gate; see [ADR 0002 — The risk engine is the only gate](adr/0002-risk-engine-is-the-only-gate.md).

## Self-custody penalty ladder

Track 1 penalizes any erosion of self-custody; fully self-custodial agents sit in
the top band. Guardrail Alpha targets that top band:

| Band | Behavior | Guardrail Alpha |
|---|---|---|
| **Top — fully self-custodial** | Keys + signing authority remain with the user's wallet end to end; the agent only proposes. | **This design.** Engine builds intents/approvals; TWAK holds keys and signs; `custodial_signing` forbidden. |
| Penalized | Agent holds delegated keys or co-signs | Not used — no key material in process. |
| Heavily penalized | Agent custodies funds / signs autonomously without the user's wallet | Structurally impossible: `execute_swap` requires an engine-minted `ApprovedOrder` and all signing is in TWAK. |

Because the top-band guarantee is enforced by the type system and the dependency
graph (not just convention), it cannot be silently regressed by a future code
change without breaking compilation.
