# ADR 0002 — The risk engine is the only gate to execution

- Status: Accepted
- Date: 2026-06-13

## Context

A strategy that can reach the executor directly is one bug away from a bad
trade. We need a single, auditable choke point that every order must pass.

## Decision

**No `RiskDecision::Approved` (or `Clipped`) ⇒ no TWAK swap.** Every order is
evaluated twice:

1. **Pre-trade** — allowlist, position cap, new-position cap, daily-loss gate,
   total-drawdown gate, stable-reserve floor, security flags, trade-frequency.
2. **Final** — re-runs the gates with the live quote attached (slippage,
   quoted liquidity).

The strategy engine produces *intent only*; it cannot sign or call TWAK. The
kill switch and drawdown throttle sit in the runtime loop ahead of order
processing. Forbidden actions (launch token, custodial signing, bypass TWAK,
trade non-eligible assets) are enumerated in the policy and rejected.

## Consequences

- The dependency graph forbids `strategy-engine → twak-client` and
  `execution → (skip risk-engine)`.
- Every approval/rejection/clip is recorded as an event, making "why did it
  trade?" fully replayable (see ADR 0004).
- Risk limits are data (`RiskPolicy`), compiled and hashed, not code — they can
  be changed and proven without recompiling the engine.
