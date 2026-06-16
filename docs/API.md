# Guardrail API — Route Index

A human-readable index of the Guardrail Alpha read-only HTTP API. Every route is
a side-effect-free `GET`: it never mutates the live book or the append-only event
log. Responses are JSON unless noted as text/plain or text/markdown. The
authoritative route list is
[`apps/guardrail-api/src/server.rs`](../apps/guardrail-api/src/server.rs)
(`build_app`); the machine-readable contract is
[api/openapi.yaml](api/openapi.yaml) (OpenAPI 3.1).

Run the server and hit any route:

```bash
cargo run -p guardrail-api                       # binds 127.0.0.1:8080
curl -fsS http://127.0.0.1:8080/health
```

There are **72 routes**. Everything is offline-safe (paper mode, deterministic
mocks). The dashboard, web-lite cockpit, and the TS/Python/Go SDKs are all
read-only consumers of this surface.

---

## Operational & health

| Route | Description |
|---|---|
| `GET /health` | Liveness and event-store visibility (event count). |
| `GET /readiness` | Readiness probe — whether the service can serve requests. |
| `GET /version` | Service name, semantic version, build target, run mode, and uptime. |
| `GET /metrics` | Prometheus-style operational metrics (kill switch, drawdown, …). |
| `GET /ops` | Operational summary for the operator. |
| `GET /events` | Recent entries from the append-only event log. |
| `GET /heartbeat` | Track-1 daily-trade heartbeat status. |
| `GET /cockpit` | Aggregated cockpit snapshot for dashboards. |

## Portfolio, signals & risk

| Route | Description |
|---|---|
| `GET /portfolio` | Current book: positions, NAV, stable reserve. |
| `GET /trades` | Confirmed trades (every one passed the risk gate + TWAK). |
| `GET /signals` | Strategy signals / intents for the current cycle. |
| `GET /risk` | Risk-engine state: drawdown, kill switch, gate decisions. |
| `GET /alerts` | Active operator alerts (staleness, drawdown, kill switch). |
| `GET /exposure` | Category / asset exposure from the latest run report. |
| `GET /regime` | Current market-regime classification and sizing exposure. |
| `GET /rebalance` | What-if rebalance plan for a preset (`?preset=`, `?nav_usd=`). |

## Reports & journal

| Route | Description |
|---|---|
| `GET /report` | Run report as JSON. |
| `GET /report/markdown` | Run report rendered as Markdown (text/markdown). |
| `GET /export/submission.md` | DoraHacks submission export (text/markdown). |
| `GET /journal` | Decision-journal projection of the agent's per-cycle reasoning. |
| `GET /briefing` | Judge/operator briefing claims and demo commands. |
| `GET /prizes` | Live prize/category evidence map (mirrors PRIZE_MAP.md). |
| `GET /scorecard` | Judge-facing weighted submission scorecard. |
| `GET /compete` | Competition registration / status evidence. |

## Config, policy & universe

| Route | Description |
|---|---|
| `GET /policy` | Active risk policy. |
| `GET /policy/compile` | Compile a natural-language mandate into a policy + hash. |
| `GET /universe` | The 20-asset eligible BSC universe. |
| `GET /config` | Configuration inventory (which configs are loaded). |
| `GET /mandates` | Configured natural-language mandates and their hashes. |
| `GET /wallet-controls` | Self-custody wallet and spender control status. |
| `GET /exit-triggers` | Configured exit triggers evaluated against positions. |

## Analytics

| Route | Description |
|---|---|
| `GET /backtest` | Backtest the live strategy over a synthetic path (`?preset=`, `?steps=`). |
| `GET /walkforward` | Walk-forward analysis across sentiment-driven windows. |
| `GET /sweep` | Scenario / sentiment sweep results. |
| `GET /optimize` | Portfolio-optimizer target weights. |
| `GET /scenarios` | Stress-scenario definitions and projected outcomes. |
| `GET /experiments` | Saved named backtest experiments. |
| `GET /ensemble` | Regime-routed ensemble meta-allocator config + current weights. |
| `GET /skills` | Track-2 strategy skill catalog (typed projection of `skills/INDEX.json`). |
| `GET /skill` | The packaged headline CMC strategy-skill descriptor. |
| `GET /funding` | Funding-rate-proxy snapshot across markets. |
| `GET /history` | Historical run/equity series. |

## Trading & market data

| Route | Description |
|---|---|
| `GET /assets` | Per-asset detail across the eligible universe. |
| `GET /indicators` | Classic technical indicators over a deterministic series. |
| `GET /quotes` | Swap quotes (price impact + slippage) per market. |
| `GET /costs` | Gas + slippage cost estimates for TWAK routes. |
| `GET /drift` | Weight drift between current book and a fresh target. |
| `GET /liquidity` | Liquidity capacity and pool usage for eligible assets. |
| `GET /trending` | Trending / attention ranking of assets. |
| `GET /watchlist` | Assets ranked by current attention needs. |
| `GET /budget` | Daily execution budget and gas-runway status. |
| `GET /playbook` | Selected operator playbook from run state. |
| `GET /job-simulator` | Simulated ERC-8183 job lifecycle against a service. |

## Agent identity, services & SDK

| Route | Description |
|---|---|
| `GET /proof` | On-chain proof commitments (policy/report hashes, BscScan links). |
| `GET /agent-card` | ERC-8004-style Guardrail agent card. |
| `GET /.well-known/agent-card.json` | Well-known agent card (discovery path). |
| `GET /agent-services` | ERC-8183 provider service offerings. |
| `GET /audit-manifest` | Submission-artifact and operator-route inventory. |
| `GET /bnb-sdk` | BNB Agent SDK module + contract mapping evidence. |
| `GET /sdk-catalog` | Product-owned BNB Agent SDK integration tree. |
| `GET /signing-policy` | x402 / EIP-712 signing-policy envelope (caps, allow/deny). |
| `GET /commerce` | ERC-8183 commerce/provider readiness mapping. |

---

## Sample responses (headline routes)

### `GET /version`

```json
{
  "service": "guardrail-api",
  "version": "0.1.0",
  "build_target": "aarch64-macos",
  "mode": "paper",
  "uptime_seconds": 42,
  "uptime_human": "0h 0m 42s"
}
```

(`mode` reads `GUARDRAIL_MODE`, default `paper`; `build_target` is `arch-os`.)

### `GET /skills`

```json
{
  "index_path": "skills/INDEX.json",
  "count": 5,
  "ids": [
    "cmc-regime-routed-alpha",
    "funding-rate-carry",
    "mean-reversion-chop",
    "trend-breakout-momentum",
    "volatility-targeted-risk-parity"
  ],
  "skills": [
    {
      "id": "cmc-regime-routed-alpha",
      "name": "regime-routed-bsc-alpha",
      "regimes": ["risk_on", "risk_off", "chop", "breakout"],
      "eligible_universe_size": 20,
      "examples_count": 4,
      "spec_file": "skills/cmc-regime-routed-alpha/strategy_spec.yaml"
    }
  ]
}
```

(`count`/`ids` reflect the registered entries in `skills/INDEX.json`; the
standalone `social-sentiment-momentum-bsc` skill directory also exists on disk.)

### `GET /ensemble`

```json
{
  "name": "guardrail-regime-ensemble",
  "version": "1.0.0",
  "reserve_symbol": "USDT",
  "current_regime": "chop",
  "active_weights": {
    "cmc-regime-routed-alpha": 0.30,
    "funding-rate-carry": 0.12,
    "mean-reversion-chop": 0.50,
    "trend-breakout-momentum": 0.08
  },
  "skills": [
    { "id": "cmc-regime-routed-alpha", "label": "general regime-routed alpha" }
  ],
  "regimes": [
    { "regime": "risk_on", "weights": { "cmc-regime-routed-alpha": 0.35 } }
  ]
}
```

### `GET /journal`

```json
{
  "total_events": 128,
  "total_cycles": 3,
  "run_ids": ["run-2026-06-15T00:00:00Z"],
  "confirmed_trades_total": 4,
  "cycles": [
    {
      "index": 0,
      "regime": "risk_on",
      "scored_assets": [ { "symbol": "WBNB", "score": 0.72 } ],
      "orders": [ { "from": "USDT", "to": "WBNB", "amount_usd": 250.0 } ],
      "nav_usd": 10050.0
    }
  ]
}
```

### `GET /health`

```json
{ "status": "ok", "events_visible": 128, "db_reachable": true }
```

> All response shapes above are illustrative; the API assembles dynamic JSON from
> event payloads and run reports, so exact fields vary. The OpenAPI spec models
> them as loose objects with the documented top-level fields.

---

## See also

- [api/openapi.yaml](api/openapi.yaml) — OpenAPI 3.1 spec for all 72 routes.
- [api/README.md](api/README.md) — guide to the spec and the read-only surface.
- [API_CLIENTS.md](API_CLIENTS.md) — every client option for this API.
- [CLI.md](CLI.md) — the command-line surface that mirrors much of this API.
