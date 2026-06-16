# Prize Map — Evidence Table

Every prize this submission targets, mapped to concrete evidence in this repo:
the criterion, where it is implemented (file / endpoint / command), and how a
judge verifies it. All "how to verify" commands run from the repo root in paper
mode (deterministic mocks, offline). A live evidence version of this table is
also served at **`GET /prizes`** (`apps/guardrail-api/src/prizes.rs`).

Companion docs: [PITCH.md](PITCH.md) · [JUDGE_DEMO.md](JUDGE_DEMO.md) ·
[WHATS_NEW.md](WHATS_NEW.md) · [HACKATHON.md](HACKATHON.md) · [TRACK2.md](TRACK2.md).

> **What changed recently:** seven Track-2 skill directories + a regime ensemble
> meta-allocator (now also a native `GET /ensemble` route) + a skill authoring kit
> (Track 2); four new read-only API routes (`/journal`, `/ensemble`, `/skills`,
> `/version`) bringing the surface to **68 routes**; a modular `guardrail-cli`
> `commands/` tree and live TUI regime/positions/risk/alerts panels; the MCP
> server now exposes tools **and** resources **and** prompts with a `manifest.json`
> (CMC Agent Hub); an independent stdlib-only on-chain proof verifier (BNB SDK); a
> self-custody demo with an example signing policy (TWAK); and Vercel auto-deploy
> for the dashboard. See [WHATS_NEW.md](WHATS_NEW.md), [CLI.md](CLI.md), and
> [API.md](API.md) for the details.

---

## Track 1 — Autonomous Trading Agents ($24k)

| Criterion | Where it is implemented | How to verify |
|---|---|---|
| **Autonomous loop** — runs unattended, full cycle | `crates/agent-runtime/src/runtime.rs` (`process_order`, per-cycle market→regime→risk→quote→execute→reconcile); binary `apps/guardrail-agent` | `GUARDRAIL_CYCLES=3 cargo run -p guardrail-agent -- --config configs/paper.toml` |
| **Verifiable risk control** — risk is the *sole* gate | `crates/risk-engine`; `TwakExecutor::execute_swap(&ApprovedOrder)` — approval minted only by risk engine (`crates/twak-client/src/{lib,approvals,swap}.rs`) | Inspect signatures; `cargo run -p guardrail-replay -- summary` shows proposed vs **rejected** vs confirmed |
| **Capital preservation under 30% DQ** | Soft throttle `max_total_drawdown_pct=22`, hard `kill_switch_drawdown_pct=24`; `kill_switch::should_trigger`, `risk-engine/src/checks/total_drawdown.rs` | `configs/risk_policy.paper.json`; `KillSwitchTriggered` event in `guardrail-replay -- journal`; `GET /risk` |
| **Kill switch (engages + stays engaged)** | `risk-engine` kill switch; CLI `kill-switch`; `apps/guardrail-monitor` watchdog | `cargo run -p guardrail-cli -- kill-switch --reason "demo"`; `GET /metrics` flips `guardrail_kill_switch=1` |
| **≥1 trade/day** | `policy.daily_trade_requirement`; idle-cycle heartbeat emits `DailyTradeRequirementSatisfied` (`runtime.rs` ~line 409) | Run the agent; see `DailyTradeRequirementSatisfied` in the journal; `GET /heartbeat` |
| **On-chain registration** | `register_competition()` (`runtime.rs` ~line 114); contract `0x212c61b9b72c95d95bf29cf032f5e5635629aed5` in `crates/common/src/constants.rs`; CLI `register` | `cargo run -p guardrail-cli -- register`; `./scripts/compete.sh`; `GET /proof` |
| **Eligible universe (20 BSC assets)** | `configs/eligible_assets.bsc.json` (20 × `chain_id 56`); `market-data::Universe`; allowlist check `risk-engine/src/checks/asset_allowlist.rs` | `cargo run -p guardrail-cli -- markets`; `curl -fsS http://127.0.0.1:8080/universe` |
| **Hold non-zero in-scope balance** | `PortfolioState::seed_stable` ($10k USDT); `min_stable_reserve_pct`; `checks/stable_reserve.rs`, `checks/wallet_balance.rs` | `GET /portfolio`; `cargo run -p guardrail-replay -- summary` |
| **Diversified, reproducible trades** | `crates/strategy-engine` score-proportional sizing capped at `max_position_weight_pct`; routes via `USDT` | `cargo run -p guardrail-replay -- trades`; `GET /trades` |
| **Explainable autonomy** — reconstruct *why* the agent acted | Decision-journal projection over the append-only log | `apps/guardrail-api/src/journal.rs`; `curl -fsS http://127.0.0.1:8080/journal`; live TUI panels (`apps/guardrail-tui/src/{regime,positions,risk,alerts}.rs`) |
| **Operability / observability** | Version + build target + uptime route; live terminal cockpit | `curl -fsS http://127.0.0.1:8080/version`; `cargo run -p guardrail-tui` |
| **Demo / reproducibility** | `scripts/demo.sh` (offline E2E), `scripts/compete.sh` (live) | `./scripts/demo.sh` |

## Track 2 — Strategy Skills ($6k)

Now **six** regime-aware Track-2 strategy skill directories on disk (five
registered in [`skills/INDEX.json`](../skills/INDEX.json) plus the standalone
`social-sentiment-momentum-bsc`), a **regime ensemble meta-allocator** that
blends them — exposed both as a Python blender and a native
**`GET /ensemble`** route — and a **skill authoring kit** so a judge can
scaffold and lint another skill in seconds. The catalog is also served as a
typed projection at **`GET /skills`** (`apps/guardrail-api/src/skills.rs`).
Every skill is advisory-only — the Rust risk engine stays the sole execution
gate.

| Skill / Surface | Criterion | Where it is implemented | How to verify |
|---|---|---|---|
| **`regime-routed-bsc-alpha`** (general alpha) | Backtestable, regime-routed strategy authored as an LLM Skill; advisory-only, Rust-validated | `skills/cmc-regime-routed-alpha/` (`skill.yaml`, `strategy_spec.yaml`, `prompts/`, `examples/`, `tests/`); served at `GET /skill` | `cargo run -p guardrail-cli -- backtest --config configs/paper.toml`; `cat skills/cmc-regime-routed-alpha/strategy_spec.yaml` |
| **`funding-rate-carry-bsc`** | Funding-rate / basis carry tilt, same risk envelope, advisory-only | `skills/funding-rate-carry/` (`skill.yaml`, `strategy_spec.yaml`, `SKILL.md`, `examples/`, `tests/`); `GET /funding` | `cat skills/funding-rate-carry/skill.yaml`; `curl -fsS http://127.0.0.1:8080/funding` |
| **`mean-reversion-chop-bsc`** | Range-fade specialist (RSI + Bollinger %B + ATR stops), peaks in the CHOP regime | `skills/mean-reversion-chop/` (`skill.yaml`, `strategy_spec.yaml`, `SKILL.md`, `examples/`, `tests/`) | `cat skills/mean-reversion-chop/strategy_spec.yaml`; `bash scripts/lint_skills.sh` |
| **`trend-breakout-momentum-bsc`** | Momentum/breakout specialist (EMA stack + MACD + Donchian + volume confirm), peaks in the BREAKOUT regime | `skills/trend-breakout-momentum/` (`skill.yaml`, `strategy_spec.yaml`, `SKILL.md`, `examples/`, `tests/`) | `cat skills/trend-breakout-momentum/strategy_spec.yaml`; `bash scripts/lint_skills.sh` |
| **`volatility-targeted-risk-parity-bsc`** | Risk-based *sizing* axis: inverse-vol equal-risk weights scaled to a 45% target portfolio vol; de-risks in risk_off | `skills/volatility-targeted-risk-parity/` (`skill.yaml`, `strategy_spec.yaml`, `examples/`, `tests/`) | `cat skills/volatility-targeted-risk-parity/strategy_spec.yaml` |
| **`social-sentiment-momentum-bsc`** | Attention axis: trending-rank velocity + volume surge + sentiment, gated on Fear & Greed; fades hype without volume | `skills/social-sentiment-momentum/` (`skill.yaml`, `prompts/`, `examples/`, `tests/`) | `cat skills/social-sentiment-momentum/skill.yaml` |
| **Skill catalog API** | Typed projection of the catalog (count, ids, full index entries) | `apps/guardrail-api/src/skills.rs`; reads `skills/INDEX.json` | `curl -fsS http://127.0.0.1:8080/skills` |
| **Regime ensemble** meta-allocator | Blends the four core skills by classified regime; exposed as Python blender **and** native API route | Config `skills/ensemble.json`; blender `python-lab/guardrail_lab/ensemble.py`; route `apps/guardrail-api/src/ensemble.rs` | `cat skills/ensemble.json`; `python3 python-lab/analyze.py ensemble --regime chop`; `curl -fsS http://127.0.0.1:8080/ensemble` |
| **Skill authoring kit** | Reproducible scaffold + lint so judges (or a host LLM) can add a skill | Template `skills/_template/`; scaffolder `scripts/new_skill.sh`; example validator `scripts/lint_skills.sh` (runs `guardrail_lab.skill` over each `examples/`) | `ls skills/_template`; `bash scripts/new_skill.sh demo-skill` then `bash scripts/lint_skills.sh` |
| All skills | Backtest reuses the **production** strategy → risk → portfolio path | `crates/backtester`; CLI `backtest` / `walk-forward`; `guardrail-sim` | `cargo run -p guardrail-cli -- walk-forward --windows 6 --steps 30`; `cargo run -p guardrail-sim`; `GET /backtest`, `/walkforward`, `/sweep` |
| All skills | Skill output-contract tests | `skills/*/tests/test_strategy_schema.json`, `test_outputs.json` | inspect the test JSON files; `bash scripts/lint_skills.sh` |

Full write-up: [TRACK2.md](TRACK2.md) · ensemble detail: [ENSEMBLE.md](ENSEMBLE.md) ·
authoring kit: [SKILL_AUTHORING.md](SKILL_AUTHORING.md).

## TWAK Special — Best Use of TWAK ($2k)

| Criterion | Where it is implemented | How to verify |
|---|---|---|
| **Self-custody signing** — engine never holds keys; TWAK signs | `crates/twak-client` (`swap.rs`, `wallet.rs`, `x402.rs`); `execute_swap(&ApprovedOrder)` requires an engine-minted approval | [SELF_CUSTODY.md](SELF_CUSTODY.md); [adr/0003-twak-only-execution.md](adr/0003-twak-only-execution.md); read `crates/twak-client/src/lib.rs` |
| **Sole execution layer** | `TwakExecutor` is the only execute path (`runtime.rs::process_order`); transports Mock/REST/MCP/CLI (`mock.rs`, `rest.rs`, `mcp.rs`, `cli.rs`) | `cargo run -p guardrail-replay -- trades` (every confirmed swap went through TWAK) |
| **x402 signing** | `crates/twak-client/src/x402.rs`; example signing policy `configs/signing_policy.example.json` (caps, allowed/forbidden actions, x402 primaryType allow/deny-list) | inspect `x402.rs`; `cat configs/signing_policy.example.json`; `GET /signing-policy` |
| **Self-custody demo** — narrated, offline, never loads keys | `scripts/self_custody_demo.sh` walks agent-proposes → risk-gates → TWAK-signs-with-user-keys → execute/reconcile, citing the real files/routes | `bash scripts/self_custody_demo.sh`; [TWAK_SELF_CUSTODY_DEMO.md](TWAK_SELF_CUSTODY_DEMO.md) |
| **Autonomous + guardrails + competition register** | `register_competition()`; dual risk gate + kill switch around every swap | `cargo run -p guardrail-cli -- register`; `GET /commerce`, `/wallet-controls` |
| Detail | — | [TWAK_INTEGRATION.md](TWAK_INTEGRATION.md) · [SELF_CUSTODY.md](SELF_CUSTODY.md) |

## CMC Special — Best Use of Agent Hub ($2k)

| Criterion | Where it is implemented | How to verify |
|---|---|---|
| **MCP server** to the CMC AI Agent Hub — full capability surface | `clients/mcp/` (`run.py`, `manifest.json`, `mcp.json`, `guardrail_mcp/`) now exposes **tools + resources + prompts** (`capabilities: {tools, resources, prompts}`); client transport `crates/cmc-client/src/mcp.rs` | `cat clients/mcp/manifest.json` (14 tools, 5 resources, 3 prompts); `cat clients/mcp/README.md` |
| **Hub-ready manifest** | Single descriptor a host reads to register the server: protocol/transport, runtime command, env, and the tool/resource/prompt catalog | `clients/mcp/manifest.json` | `cat clients/mcp/manifest.json` |
| **CMC integration** — all data methods | `crates/cmc-client` — `CmcDataSource` trait (quotes, OHLCV, Fear & Greed, DEX liquidity, token security, trending, global) via REST/MCP/x402/Mock (`rest.rs`, `mcp.rs`, `x402.rs`, `mock.rs`, `endpoints.rs`) | [CMC_INTEGRATION.md](CMC_INTEGRATION.md); `GET /quotes`, `/trending`, `/liquidity`, `/indicators` |
| **x402 pay-and-retry** for paid CMC requests | `crates/cmc-client/src/x402.rs` + `retry.rs` | inspect `x402.rs`; `compete.sh` env checklist (`CMC_X402_*`) |
| **Packaged CMC Skill** | `skills/cmc-regime-routed-alpha` consumes the CMC inputs | `GET /skill`; [TRACK2.md](TRACK2.md) |
| **CMC data → capability lineage** — verifiable, discoverable | `configs/cmc/capabilities.json` declares each CMC dataset (quotes/OHLCV/Fear&Greed/DEX-liquidity/security/trending/global) → the read-only capability it powers, with the exact `cmc-client` source and the API route / MCP tool that exposes it. Served at `GET /cmc/capabilities`, mirrored as MCP resource `guardrail://cmc/capabilities`, advertised in the ERC-8004 agent card. **No execution is exposed to the hub.** | `curl -fsS http://127.0.0.1:8080/cmc/capabilities`; [CMC_AGENT_HUB.md](CMC_AGENT_HUB.md) |

## BNB Special — Best Use of BNB AI Agent SDK ($2k)

| Criterion | Where it is implemented | How to verify |
|---|---|---|
| **Agent identity crate** | `crates/bnb-agent` — `AgentIdentity`, `AgentMetadata` (`identity.rs`, `metadata.rs`) | `cargo run -p guardrail-cli -- identity --config configs/paper.toml`; `GET /bnb-sdk`, `/agent-card` |
| **ERC-8004 / 8183 records** | `crates/bnb-agent/src/{erc8004,erc8183}.rs`, `registration.rs` | inspect the record in `identity` output; `GET /.well-known/agent-card.json` |
| **On-chain proof + commitments** | `proof.rs`, `report_hash.rs` — `AgentProof`, `policy_hash`, `report_hash`, BscScan links; wired into runtime start/report events | `cargo run -p guardrail-cli -- identity`; `GET /proof` |
| **Independent proof verifier** — "don't trust, verify" | `clients/proof-verifier/` — a stdlib-only, clean-room Python tool that re-derives `policy_hash`, `report_hash`, `agent_id`, `address_url`, and contract/tx URL formats from first principles and compares them to the claimed proof. Shares no code with the Rust agent. | `bash scripts/verify_proof.sh` (auto-selects run report or the bundled offline fixture); `python3 clients/proof-verifier/verify.py --strict`; [PROOF_VERIFICATION.md](PROOF_VERIFICATION.md) |
| **On-chain verification** — verifiable, not self-attested | `crates/chain-verifier` — read-only BSC JSON-RPC (`eth_chainId`, `eth_getCode`, `eth_getTransactionReceipt`) confirming the competition contract is deployed and the registration tx was mined. Wired into `GET /proof/verify` and the dashboard `/proof` page; mirrored in the standalone verifier (`--rpc`). ABI-free, no keys, offline-safe (`skipped` when `BSC_RPC_URL` is unset). | `python3 clients/proof-verifier/verify.py --rpc https://bsc-dataseed.bnbchain.org` (confirms ~2.5 KB of contract bytecode on BSC mainnet); `BSC_RPC_URL=… curl -fsS http://127.0.0.1:8080/proof/verify` |
| Detail | — | [BNB_AGENT_IDENTITY.md](BNB_AGENT_IDENTITY.md) |

---

## Quick verification (offline, one block)

```bash
# Full pipeline, all evidence in one run
./scripts/demo.sh

# Or spot-check the live evidence map + key surfaces
cargo run -p guardrail-api &
curl -fsS http://127.0.0.1:8080/prizes      # this table, with live run facts
curl -fsS http://127.0.0.1:8080/universe    # 20 eligible BSC assets
curl -fsS http://127.0.0.1:8080/proof       # BNB identity + registration proof
curl -fsS http://127.0.0.1:8080/skill       # Track 2 skill descriptor
curl -fsS http://127.0.0.1:8080/funding     # carry Track 2 skill
curl -fsS http://127.0.0.1:8080/scenarios   # stress scenario library
curl -fsS http://127.0.0.1:8080/signing-policy   # TWAK self-custody envelope
```

Recently shipped surfaces, each verifiable offline in one command:

```bash
python3 python-lab/analyze.py ensemble --regime chop   # regime ensemble blend
python3 python-lab/analyze.py journal                   # decision journal
bash scripts/lint_skills.sh                             # validate all 4 skills' examples
bash scripts/verify_proof.sh                            # independent on-chain proof check
bash scripts/self_custody_demo.sh                       # TWAK self-custody walkthrough
cat clients/mcp/manifest.json                           # MCP tools+resources+prompts
```

**Total targeted: $24k (Track 1) + $6k (Track 2) + $6k (3 × $2k specials) = $36k.**
