# CMC Agent Hub

Guardrail Alpha is a **CMC-powered agent**: every market decision is derived from
CoinMarketCap data. This document makes that integration *verifiable and
discoverable* — the gap between "we call CMC" and "here is exactly which CMC
dataset powers which capability, and how to call it."

## The lineage descriptor

`configs/cmc/capabilities.json` is the single source of truth. It declares, for
each CMC dataset the agent consumes:

- the CMC endpoint family,
- the exact `cmc-client` source (`crates/cmc-client/src/rest.rs::<method>`),
- which read-only capability it powers.

And for each exposed capability: its CMC inputs, the API route, and the MCP
tool / resource that exposes it.

| CMC dataset | Powers |
|---|---|
| `latest_quotes` | alpha scoring, NAV marking, momentum |
| `ohlcv` | momentum, volatility, indicators |
| `fear_and_greed` | market regime, sentiment |
| `dex_liquidity` | liquidity gate, execution quality |
| `token_security` | security gate, risk penalty |
| `trending` | watchlist, discovery |
| `global_metrics` | market regime, BTC dominance |

## Discovery surfaces

A CMC Agent Hub consumer can discover and call the agent three ways, all
read-only:

- **HTTP** — `GET /cmc/capabilities` serves the descriptor plus a summary
  (dataset count, capability count, `execution_exposed: false`).
- **MCP** — resource `guardrail://cmc/capabilities` mirrors the descriptor; the
  existing tools (`guardrail_regime`, `guardrail_skill`, `guardrail_backtest`,
  `guardrail_liquidity`, `guardrail_compile_policy`) are the callable capabilities.
- **Agent card** — `GET /.well-known/agent-card.json` advertises a `CMC Agent Hub`
  service entry pointing at the capabilities endpoint, so the agent is
  discoverable from its ERC-8004 identity document.

## Execution boundary (important)

The hub surface is **analysis only**. The agent never exposes trade execution,
signing, or fund movement to the hub — that would violate the project's core
invariant. The Rust risk engine remains the sole execution gate, reachable only
by the internal trading binary. Exposing CMC-derived *intelligence* (regime,
scores, backtests, liquidity) is safe and reusable; exposing *execution* is not,
and is deliberately absent.

## Verify

```bash
# the descriptor and its summary
curl -fsS http://127.0.0.1:8080/cmc/capabilities | jq '.summary, .descriptor.datasets[].dataset'

# discoverable from the agent card
curl -fsS http://127.0.0.1:8080/.well-known/agent-card.json | jq '.card.services[] | select(.name=="CMC Agent Hub")'
```
