# Demo Script

A literal, copy-pasteable walkthrough. All commands run from the repo root.
Paper mode uses deterministic CMC and TWAK mocks, so every step is reproducible
offline — no API keys, no chain access.

The CLI binary is `guardrail` (`apps/guardrail-cli`); the agent binary is
`guardrail-agent`.

## 0. Build

```bash
cargo build
```

## 1. Compile a natural-language mandate into a hashed policy

Turn plain English into a validated `RiskPolicy` and its SHA-256 hash — the
on-chain-publishable fingerprint of exactly what governs the agent.

```bash
cargo run -p guardrail-cli -- policy compile \
  "Trade USDT CAKE WBNB. Max drawdown 22%, daily loss 7%, max position 18%, \
   stable reserve 10%, slippage 0.8%, kill switch 24%, 1 trade per day. No leverage."
```

Prints `policy_hash: <sha256>` followed by the canonical policy JSON.

> Point at: the hash, the parsed limits, and the `allowed_assets` allowlist.

To hash an existing policy file instead:

```bash
cargo run -p guardrail-cli -- policy hash configs/risk_policy.paper.json
```

## 2. Show the market regime + alpha scores

Show the current regime, the strategy headline, top alpha scores, and the
resulting target portfolio (warms up a short synthetic path first):

```bash
cargo run -p guardrail-cli -- score --config configs/paper.toml
```

## 3. Run the paper agent (bounded cycles)

Run the autonomous runtime end to end. `GUARDRAIL_CYCLES` bounds the number of
cycles (default 4 in paper mode). Each cycle: market data → snapshot → regime →
scores → per-order risk gate → TWAK quote → final risk → mock execute →
reconcile → event log, plus the daily-trade heartbeat when a cycle is idle.

```bash
GUARDRAIL_CYCLES=3 cargo run -p guardrail-agent -- --config configs/paper.toml
```

This writes the append-only event log to `data/guardrail_alpha.db` and a
`data/run_report.json` snapshot.

> Point at: the `AgentStarted` log line carrying `agent_id`, `wallet`, and
> `policy_hash`, then the per-cycle regime → risk → quote → execute flow.

## 4. Replay the event log (the audit trail)

The replay tool is read-only — it never trades and never mutates the log.

```bash
# Chronological decision journal
cargo run -p guardrail-replay -- journal

# Confirmed on-chain swaps only
cargo run -p guardrail-replay -- trades

# Event-type counts
cargo run -p guardrail-replay -- summary

# CSV export (use "-" for stdout)
cargo run -p guardrail-replay -- export-csv data/exports/events.csv
```

> Point at: `trades` answers "why did it trade, what did it quote, what tx
> resulted?"; `summary` shows proposed vs. rejected vs. confirmed counts.

## 5. Show Prometheus metrics

Start the exporter sidecar (reads the event log + `data/run_report.json`), then
scrape it.

```bash
cargo run -p guardrail-exporter &     # binds 0.0.0.0:9100
curl -fsS http://127.0.0.1:9100/metrics
```

> Point at: `guardrail_nav_usd`, `guardrail_total_drawdown_pct`,
> `guardrail_kill_switch`, `guardrail_trades_total`, `guardrail_report_age_seconds`.

## 6. Backtest + walk-forward

Run the real strategy, risk gate, and portfolio accounting over a synthetic path
and print a Markdown metrics report (return, max drawdown, trades, win rate,
profit factor):

```bash
cargo run -p guardrail-cli -- backtest --config configs/paper.toml --steps 60
```

Walk-forward across a sentiment ramp (per-window table + aggregate):

```bash
cargo run -p guardrail-cli -- walk-forward --config configs/paper.toml --windows 6 --steps 30
```

Sweep the backtest across Fear & Greed regimes (defensive → constructive):

```bash
cargo run -p guardrail-sim
# walk-forward mode:
cargo run -p guardrail-sim -- --walk-forward
```

> Point at: how exposure, return, and drawdown shift as sentiment moves from
> fearful to greedy — the same production strategy + risk path, only the
> sentiment input varies.

## 7. Show the agent identity + registration

```bash
# Full BNB identity + proof commitments as JSON
cargo run -p guardrail-cli -- identity --config configs/paper.toml

# Track 1 competition registration target
cargo run -p guardrail-cli -- register
```

> Point at: `agent_id` (SHA-256 of name + wallet), `wallet`, `address_url`,
> `policy_hash`, and the ERC-8004 record — all deterministic, no chain calls.

## 8. Start the API and open the dashboard

```bash
cargo run -p guardrail-api &          # read-only, binds 0.0.0.0:8080
cd dashboard && pnpm install && pnpm dev   # http://localhost:3000
```

Walk these dashboard pages (all read-only):

- `/` (Cockpit) — regime, target, kill-switch state, tx count.
- `/proof` — agent id, registration tx, latest report, run report.
- `/policy` — active policy + hash. `/universe` — eligible BSC allowlist.
- `/risk`, `/trades`, `/signals`, `/events` — the live audit surfaces.
- `/readiness`, `/alerts`, `/observability` — operator status.
- `/backtest`, `/walkforward`, `/lab`, `/reports` — analytics.

Spot-check the API directly:

```bash
curl -fsS http://127.0.0.1:8080/proof
curl -fsS http://127.0.0.1:8080/policy
curl -fsS http://127.0.0.1:8080/readiness
```

## 9. Show the kill switch

The risk engine engages the kill switch when total drawdown reaches
`kill_switch_drawdown_pct` (24% in `configs/risk_policy.paper.json`); once
engaged it stays engaged and halts trading. To force a visible kill-switch
breach in a short demo, lower that threshold in the policy (e.g. to `1`) and
re-run the paper agent — the run will emit a `KillSwitchTriggered` event and
stop trading. There is also a manual operator trigger:

```bash
cargo run -p guardrail-cli -- kill-switch --reason "manual_operator_trigger"
# or: ./scripts/kill_switch.sh
```

> Point at: `guardrail-replay -- journal` shows the `KillSwitchTriggered`
> event; `/metrics` flips `guardrail_kill_switch` to `1`; `/alerts` raises it.

## 10. Export the submission artifact

```bash
./scripts/export_report.sh    # writes data/exports/submission.md
```

Pulls `GET /export/submission.md` from a running API, falling back to the local
`data/run_report.json` if the API is not up.
