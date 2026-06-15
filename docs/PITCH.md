# Guardrail Alpha — Judge Pitch

**Track 1: Autonomous Trading Agents.** A live, autonomous on-chain trading
agent in Rust whose **risk engine is the sole gate to execution** — self-custodial
by construction, fed by CoinMarketCap intelligence, and provably an agent on BNB
Chain.

---

## The problem

Autonomous on-chain trading agents fail on two things judges actually care about:

1. **Verifiable risk control.** An agent that can place trades can also blow up.
   Most agents bolt risk on as advisory checks the strategy can ignore. Track 1
   disqualifies on a 30% drawdown — "the model promised to be careful" is not a
   control.
2. **Self-custody.** An agent that holds private keys is a custody liability. If
   the strategy code can sign, a prompt-injected or buggy strategy can drain the
   wallet.

So the bar is: an agent that **cannot trade without passing risk**, and that
**cannot sign even if it wanted to**.

## The solution

Guardrail Alpha is a Rust live engine built around a single invariant:

> **The risk engine is the only path to execution, and execution lives in a
> separate signer the strategy cannot reach.**

- **Risk engine as the sole gate.** Execution (`TwakExecutor::execute_swap`)
  accepts only an engine-minted `ApprovedOrder`. The approval type is constructed
  exclusively by the risk engine, so "trade without risk approval" is a *compile
  error*, not a runtime hope. Two gates run per order — pre-quote and post-quote —
  plus a kill switch that halts on drawdown and stays engaged.
  (`crates/twak-client/src/{lib,approvals,swap}.rs`,
  `crates/agent-runtime/src/runtime.rs::process_order`).
- **TWAK self-custody signing.** The engine only builds intents and approvals;
  **TWAK holds the keys and signs**. There is no key material in the agent.
  Self-custodial by type, not by policy. (`crates/twak-client`,
  [SELF_CUSTODY.md](SELF_CUSTODY.md)).
- **CMC data via x402.** Market intelligence (quotes, OHLCV, Fear & Greed, DEX
  liquidity, token security, trending, global) flows in through a `CmcDataSource`
  trait with a REST / MCP / **x402 pay-and-retry** transport.
  (`crates/cmc-client`, [CMC_INTEGRATION.md](CMC_INTEGRATION.md)).
- **BNB identity + ERC-8004 proof.** The agent registers on BNB Chain and emits
  an ERC-8004/8183 identity record with a deterministic `agent_id`, a
  `policy_hash`, a `report_hash`, and BscScan proof links.
  (`crates/bnb-agent`, [BNB_AGENT_IDENTITY.md](BNB_AGENT_IDENTITY.md)).

## Differentiators

| Most agents | Guardrail Alpha |
|---|---|
| Risk checks are advisory; the strategy can route around them | Risk is the **type-enforced sole gate** — no approval, no `execute_swap`, enforced at compile time |
| Agent holds keys / signs locally | **TWAK signs**; the engine has zero key material — self-custody by construction |
| "Trust the model" capital preservation | Dual risk gate + drawdown kill switch that **stays engaged**, soft throttle at 22%, hard halt at 24% — both under the 30% DQ line |
| Backtest is a separate, diverging model | Backtester re-runs the **exact production** strategy → risk → portfolio code path |
| Identity is a wallet address | ERC-8004/8183 record with `policy_hash` + `report_hash` commitments and BscScan proof |
| Data is a single REST key | `CmcDataSource` trait over REST / MCP / **x402** with pay-and-retry |

## Architecture (one diagram)

```
                          GUARDRAIL ALPHA  (Rust live engine)

  ┌────────────────────────────────────────────────────────────────────────┐
  │                          agent-runtime  (autonomous loop)                │
  │                                                                          │
  │  CMC AI Agent Hub          strategy-engine          risk-engine          │
  │  ┌───────────────┐         ┌──────────────┐         ┌──────────────────┐ │
  │  │ cmc-client    │  data   │ regime route │ intent  │  GATE 1 pre-quote │ │
  │  │ REST/MCP/x402 │────────▶│ alpha scores │────────▶│  + kill switch    │ │
  │  └───────────────┘         │ target book  │         └────────┬─────────┘ │
  │   quotes OHLCV F&G          └──────────────┘                  │ approved? │
  │   liquidity security                                          ▼           │
  │   trending global                              ┌──────────────────────┐  │
  │                                          quote │  GATE 2 post-quote    │  │
  │                                       ◀────────│  → ApprovedOrder      │  │
  │                                       │        └──────────┬───────────┘  │
  │                                       │                   │ ONLY path     │
  │                                       ▼                   ▼ to execute    │
  │                              ┌─────────────────────────────────────────┐ │
  │                              │  twak-client  (self-custody)             │ │
  │                              │  execute_swap(&ApprovedOrder)  ◀─ keys   │ │
  │                              │  register_competition()        live in   │ │
  │                              └──────────────┬──────────────────────────┘ │
  │                                             │                            │
  │   bnb-agent (identity)        event-store   │  reconcile + AgentEvent log │
  │   ERC-8004/8183 + proof   ◀── append-only ──┘                            │
  └────────────────────────────────────────────────────────────────────────┘
        │                                   │                       │
        ▼                                   ▼                       ▼
   BNB Chain                          TWAK signer / DEX        SQLite event log
   register tx + proof                (holds keys, signs)      → API, replay,
   (BscScan links)                                              exporter, dashboard

  Surfaces:  guardrail-api (read-only HTTP)  ·  guardrail-monitor (watchdog)
             guardrail-replay (audit)        ·  clients/web-lite + Next.js dashboard
```

Key invariant in one line: `execute_swap` takes `&ApprovedOrder`; `ApprovedOrder`
is minted only by the risk engine; keys live only in TWAK. **No risk pass, no
trade. No TWAK, no signature.**

## Why this wins Track 1

Track 1 asks for an *autonomous trading agent* that preserves capital, stays
under the DQ drawdown line, trades at least once a day, registers on-chain, and
trades the eligible universe. Guardrail Alpha does all of it autonomously, and
makes the two hard requirements — **verifiable risk control** and **self-custody**
— structural rather than aspirational. See [HACKATHON.md](HACKATHON.md) for the
requirement-by-requirement evidence map, and [PRIZE_MAP.md](PRIZE_MAP.md) for the
full prize evidence table.

Run it in 3 minutes: [JUDGE_DEMO.md](JUDGE_DEMO.md).
