# Judge Demo — 3 to 5 Minutes

A tight, live, copy-pasteable walkthrough for judges. Everything runs in **paper
mode against deterministic CMC + TWAK mocks**, so it is fully reproducible
offline — no API keys, no chain access. The longer, exhaustive version is
[DEMO_SCRIPT.md](DEMO_SCRIPT.md); the requirement map is [HACKATHON.md](HACKATHON.md);
the prize evidence is [PRIZE_MAP.md](PRIZE_MAP.md).

Binaries used: `guardrail-agent`, `guardrail-api`, `guardrail-cli`,
`guardrail-monitor` (plus `guardrail-replay` and `guardrail-sim`). All commands
run from the repo root.

---

## Option A — one command (recommended for the live run)

```bash
./scripts/demo.sh
```

This runs the entire pipeline end to end: doctor preflight → NL→policy compile →
paper agent (3 cycles) → replay audit → Prometheus metrics → markets → backtest /
walk-forward / sweep → on-chain identity → submission report. Watch the section
banners; the talking points below tell the judge what to look for at each.

To go live for the competition (registers via TWAK, then runs against
`configs/production.toml`):

```bash
./scripts/compete.sh
```

---

## Option B — the guided 5-step live demo

### Step 1 (~30s) — Risk is the gate: compile a mandate into a hashed policy

```bash
cargo run -p guardrail-cli -- policy compile \
  "Trade USDT CAKE WBNB. Max drawdown 22%, daily loss 7%, max position 18%, \
   stable reserve 10%, slippage 0.8%, kill switch 24%, 1 trade per day. No leverage."
```

**Judge sees:** `policy_hash: <sha256>` and the canonical `RiskPolicy` JSON —
the on-chain-publishable fingerprint of exactly what governs the agent. Point at
the parsed limits and the `allowed_assets` allowlist. This hash is what the
identity record commits to in Step 4.

### Step 2 (~90s) — Run the autonomous agent (paper, bounded cycles)

```bash
GUARDRAIL_CYCLES=3 cargo run -p guardrail-agent -- --config configs/paper.toml
```

Each cycle: market data → snapshot → **regime → alpha scores** → per-order **risk
gate 1** → **TWAK quote** → **risk gate 2 → ApprovedOrder** → mock execute →
reconcile → append-only event log.

**Judge sees, in order:**
- `AgentStarted` log line carrying `agent_id`, `wallet`, and `policy_hash`, and —
  when registration is enabled — `register_competition()` producing
  **`registered=true`** with the competition contract
  `0x212c61b9b72c95d95bf29cf032f5e5635629aed5`.
- The eligible universe loaded: **20 eligible BSC assets** (curated `chain_id 56`
  subset of the Track 1 universe), from `configs/eligible_assets.bsc.json`.
- **Diversified trades** across multiple names (score-proportional, capped at the
  per-name weight limit), each routed through `USDT`.
- If a cycle would trade nothing, the runtime injects a compliant heartbeat and
  emits **`DailyTradeRequirementSatisfied`** — the ≥1-trade-per-day requirement,
  satisfied autonomously.

Writes the event log to `data/guardrail_alpha.db` and `data/run_report.json`.

### Step 3 (~45s) — The audit trail (read-only replay)

```bash
cargo run -p guardrail-replay -- summary      # proposed vs rejected vs confirmed
cargo run -p guardrail-replay -- trades       # confirmed swaps: why, quote, tx
cargo run -p guardrail-replay -- journal      # chronological decision journal
```

**Judge sees:** `summary` shows the **risk gate rejecting** some proposals and
confirming others — proof the gate is live, not cosmetic. `trades` answers "why
did it trade, what did it quote, what tx resulted?" `journal` is the full
decision narrative including any `DailyTradeRequirementSatisfied` /
`KillSwitchTriggered` events.

### Step 4 (~30s) — Kill-switch gating + on-chain identity

```bash
# Manual operator kill switch (also: ./scripts/kill_switch.sh)
cargo run -p guardrail-cli -- kill-switch --reason "judge_demo_trigger"

# BNB identity + ERC-8004 proof commitments (deterministic, no chain calls)
cargo run -p guardrail-cli -- identity --config configs/paper.toml

# Track 1 competition registration target
cargo run -p guardrail-cli -- register
```

**Judge sees:** the kill switch engages and **stays engaged** (halts trading —
the capital-preservation control under the 30% DQ line). `identity` prints
`agent_id` (SHA-256 of name + wallet), `wallet`, `address_url`, `policy_hash`,
and the **ERC-8004 record** with BscScan proof links. `register` shows the
competition registration target.

### Step 5 (~60s) — The visual: cockpit, watchdog, and dashboard

```bash
# Read-only API (binds 0.0.0.0:8080)
cargo run -p guardrail-api &

# Web-lite cockpit (zero-build, single HTML file) — open in a browser:
#   clients/web-lite/index.html   (point it at http://localhost:8080)

# Watchdog: alerts on staleness, drawdown breach, engaged kill switch
GUARDRAIL_MONITOR_CHECKS=1 cargo run -p guardrail-monitor

# Spot-check the API directly
curl -fsS http://127.0.0.1:8080/proof
curl -fsS http://127.0.0.1:8080/policy
curl -fsS http://127.0.0.1:8080/readiness
curl -fsS http://127.0.0.1:8080/universe   # the 20-asset eligible allowlist
curl -fsS http://127.0.0.1:8080/prizes     # live prize evidence map

# Full Next.js dashboard (the headline visual)
cd dashboard && pnpm install && pnpm dev    # http://localhost:3000
```

**Judge sees:** the **web-lite cockpit** (single static HTML, no build) shows
regime, target book, kill-switch state, and tx count off the live API — instant
visual with zero toolchain. Click through its tabs — the newest are
**Ensemble** (the four Track-2 skills blended by regime, with per-skill
attribution mirroring `skills/ensemble.json`), **Journal** (the per-cycle
decision narrative off `/events`), and **Signing** (the TWAK self-custody
envelope off `/signing-policy`) — alongside Backtest, Walk-forward, Funding,
Scenarios, Skill, and more. `guardrail-monitor` is the production watchdog
raising alerts (and, when `GUARDRAIL_WEBHOOK` is set, relaying them
via the `notifier` crate's webhook sink). The **Next.js dashboard** is the full
visual: `/` cockpit, `/proof` (agent id, registration tx, latest report),
`/policy` + `/universe` (active policy + hash, 20-asset allowlist), `/risk`
`/trades` `/signals` `/events` (live audit), and `/backtest` `/walkforward`
`/skill` (analytics + Track 2 skill).

### Step 6 (~75s) — The newest evidence: ensemble, journal, MCP, and proof

```bash
# Track 2: blend the four skills by the live regime (book + per-skill attribution)
python3 python-lab/analyze.py ensemble --regime chop

# Track 1/audit: human-readable per-cycle decision journal from the event log
python3 python-lab/analyze.py journal

# Track 2: scaffold + validate a skill with the authoring kit (no overwrite)
bash scripts/new_skill.sh demo-skill && bash scripts/lint_skills.sh

# CMC Agent Hub: the MCP capability handshake — tools + resources + prompts
cat clients/mcp/manifest.json
python3 clients/mcp/run.py <<<'{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"judge","version":"1.0"}}}'

# BNB SDK: independent, clean-room verification of the agent's on-chain proof
bash scripts/verify_proof.sh

# TWAK: narrated, offline self-custody walkthrough (never loads keys)
bash scripts/self_custody_demo.sh
```

**Judge sees:**
- **`analyze.py ensemble`** prints the blended target book for the chosen regime
  with per-skill contribution weights — the regime ensemble meta-allocator
  (`python-lab/guardrail_lab/ensemble.py`, config `skills/ensemble.json`),
  advisory-only on top of the same risk gate.
- **`analyze.py journal`** renders the append-only event log as a readable
  cycle-by-cycle decision narrative (regime → scores → target → risk → execute).
- **`new_skill.sh` + `lint_skills.sh`** scaffold a fifth skill from
  `skills/_template/` and then validate every skill's `examples/` with the real
  `guardrail_lab.skill` validator — the skill authoring kit in action.
- The **MCP handshake** returns an `initialize` result advertising
  `capabilities: { tools, resources, prompts }` — the server is Hub-ready, not
  tools-only. `manifest.json` lists all 14 tools, 5 resources, and 3 prompts.
- **`verify_proof.sh`** runs the stdlib-only `clients/proof-verifier/verify.py`,
  which re-derives `policy_hash`, `report_hash`, and `agent_id` from first
  principles and PASSes against the live run report or the bundled offline
  fixture — proof the identity is verifiable by anyone, not merely asserted.
- **`self_custody_demo.sh`** walks the agent-proposes → risk-gates →
  TWAK-signs-with-user-keys → execute/reconcile flow, pointing at the real files
  and the example envelope `configs/signing_policy.example.json`. No keys, no
  network.

---

## What each step proves

| Step | Track 1 / prize claim it demonstrates |
|---|---|
| 1 | Verifiable risk control — NL mandate → hashed, enforced `RiskPolicy` |
| 2 | Autonomy + `registered=true`, 20 eligible assets, diversified trades, `DailyTradeRequirementSatisfied` |
| 3 | The risk gate is real — proposals get rejected; full audit trail |
| 4 | Kill-switch gating (capital preservation) + BNB ERC-8004 identity/proof |
| 5 | Self-custody surfaces, the web-lite Ensemble/Journal/Signing tabs, and the Next.js dashboard visual |
| 6 | Track 2 ensemble + authoring kit, the MCP tools/resources/prompts handshake (CMC), the independent proof verifier (BNB), and the TWAK self-custody demo |
