# Live Competition Runbook

Operator runbook for taking **Guardrail Alpha** live during competition week.
The live profile is `configs/production.toml` and the one-command launcher is
`scripts/compete.sh`. The runtime falls back to deterministic mocks whenever a
key or URL is absent, so it never fails closed mid-window — but a real live run
requires the environment below.

Competition contract: `0x212c61b9b72c95d95bf29cf032f5e5635629aed5`

---

## 0. The one-command paths

Two turn-key scripts cover the whole lifecycle. Both auto-tick
`docs/SUBMISSION_CHECKLIST.md` from real evidence.

```bash
# Offline-safe: paper run + kill-switch demo + proof capture. No keys needed.
scripts/capture_submission.sh

# Live: spends real money. Requires CMC_API_KEY, BSC_RPC_URL, and a funded TWAK
# wallet (see §1). Preflights, confirms, registers on-chain, trades, captures.
scripts/go_live.sh
```

Before going live, run the live preflight — it refuses to green until the
credentials, live config, and conservative caps are all in place:

```bash
cargo run -q -p guardrail-doctor -- --live
```

`scripts/go_live.sh` runs this preflight for you, then the independent on-chain
verifier against `BSC_RPC_URL`, and only then asks for a typed `GO LIVE`
confirmation before placing any real trade.

---

## 1. Required environment

Export these before launch (in `.env` or the shell). See `.env.example`.

| Variable | Required | Purpose |
|---|---|---|
| `CMC_API_KEY` | yes | CoinMarketCap API key for the REST data path |
| `TWAK_BASE_URL` | yes | Base URL of the TWAK REST execution surface (resolves `[twak].base_url`) |
| `BSC_RPC_URL` | yes | BNB Smart Chain JSON-RPC endpoint |
| `CMC_X402_FROM` | optional | Payer address for x402-settled CMC requests |
| `CMC_X402_SIGNATURE` | optional | Signature authorizing the x402 payment |

If any required var is missing, the agent stays on offline mocks (safe, but not
competing). Confirm everything is set with the checklist printed by
`scripts/compete.sh`.

```bash
cargo run -q -p guardrail-doctor      # preflight: configs parse + readiness
```

---

## 2. Register before the trading window opens

Registration must land **before** the trading window opens — treat the window
open as a hard deadline and register at least a few hours early so a failed
attempt can be retried. Registration is one-time per competition.

```bash
# Via the launcher (recommended), or standalone:
cargo run -q -p guardrail-cli -- register --transport rest --autonomous true
```

- With `TWAK_BASE_URL` set, this self-submits the registration over REST and
  prints the wallet address, transport, and competition contract.
- With `TWAK_BASE_URL` unset, it falls back to the offline mock and prints the
  manual self-custody fallback: run `twak compete register` by hand.

Verify the agent identity / proof commitments any time:

```bash
cargo run -q -p guardrail-cli -- identity --config configs/production.toml
```

---

## 3. Start the live agent

```bash
./scripts/compete.sh            # doctor -> register -> contract -> live agent
```

Or start the agent directly once registered:

```bash
cargo run -p guardrail-agent -- --config configs/production.toml
```

The launcher prints the env checklist, runs preflight, registers, echoes the
contract, then `exec`s the agent so Ctrl-C cleanly stops the process.

---

## 4. Monitoring

Keep these running for the duration of the window.

```bash
# Prometheus metrics exporter (NAV, trades, drawdown, positions, kill switch)
EXPORTER_ADDR="127.0.0.1:9109" cargo run -p guardrail-exporter
curl -fsS http://127.0.0.1:9109/metrics | grep -E "guardrail_(nav_usd|trades_total|total_drawdown_pct|positions|kill_switch)"

# Live operator monitor
cargo run -p guardrail-monitor

# Read-only API + dashboard
DATABASE_URL=sqlite://data/guardrail_alpha.db cargo run -p guardrail-api   # :8080
(cd dashboard && pnpm dev)                                                 # :3000
```

Operator API surfaces: `/cockpit`, `/alerts`, `/readiness`, `/metrics`,
`/policy`, `/universe`, `/config`, `/ops`, `/report`. The dashboard mirrors
these at `/alerts`, `/readiness`, `/events`, `/policy`, `/observability`,
`/reports`, and `/proof`.

Watch `/alerts` for freshness, slippage, drawdown, daily-trade, and kill-switch
alerts.

---

## 5. Kill switch

Trigger an immediate halt if drawdown or operational risk demands it:

```bash
cargo run -q -p guardrail-cli -- kill-switch --reason "drawdown breach"
```

Our policy kill switch is wired at **24% drawdown** — a deliberate margin below
the competition disqualification threshold (see below). When the kill switch
trips, stop accepting new positions and investigate before restarting.

---

## 6. Competition requirements

| Requirement | Rule | Our margin |
|---|---|---|
| Daily trade | At least **1 trade per day** | Strategy loop runs every 15 min; heartbeat + `/alerts` daily-trade alert flag a stale day before it counts against us |
| Drawdown DQ | Disqualified at **>= 30% drawdown** | Kill switch trips at **24%**, a 6-point safety margin |

**Daily-trade heartbeat:** monitor the daily-trade alert on `/alerts` (and the
dashboard). If a day is at risk of zero trades, intervene — the alert fires
ahead of the deadline so there is time to act.

**Drawdown DQ:** `guardrail_total_drawdown_pct` on the exporter and the
drawdown alert track the live number. Treat 24% as the operator hard stop; the
kill switch enforces it automatically.

---

## 7. Recovery and restart

1. Identify the trigger: check `/alerts`, the exporter metrics, and the agent
   logs (`RUST_LOG=info`).
2. If the kill switch tripped on a transient issue (RPC outage, data
   freshness), resolve the root cause first.
3. Re-run preflight before restarting:
   ```bash
   cargo run -q -p guardrail-doctor
   ```
4. Restart the agent (registration persists — do **not** re-register):
   ```bash
   cargo run -p guardrail-agent -- --config configs/production.toml
   ```
5. Confirm trades resume and the daily-trade requirement is still satisfiable
   for the current day; if the day is nearly over, ensure at least one trade
   lands.

State (NAV, positions, events) lives in `sqlite://data/guardrail_alpha.db` and
survives restarts, so the agent resumes from where it left off.
