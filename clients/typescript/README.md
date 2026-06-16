# @guardrail/client

Typed, dependency-free TypeScript client for the Guardrail Alpha read-only API.

Works in Node 18+ and the browser (uses the global `fetch`). The API is
read-only; this client never mutates agent state.

## Usage

```ts
import { GuardrailClient } from "@guardrail/client";

const client = new GuardrailClient({ baseUrl: "http://localhost:8080" });

// Live state
const health = await client.health();
const alerts = await client.alerts();
const nav = await client.history();

// Research
const bt = await client.backtest({ steps: 60, fearGreed: 70, preset: "balanced" });
console.log(bt.metrics.total_return_pct, bt.excess_return_pct);

const wf = await client.walkforward({ windows: 6, steps: 30, preset: "aggressive" });
const sweep = await client.sweep({ steps: 40, fearGreed: [20, 50, 80] });

// Natural-language policy
const compiled = await client.compilePolicy(
  "Trade CAKE and WBNB, max drawdown 20%, kill switch 25%, stable reserve 10%",
);
console.log(compiled.hash, compiled.policy);

// Typed governance + competition reads
const regime = await client.regime();        // RegimeResponse
const funding = await client.funding();       // FundingResponse
const scenarios = await client.scenarios();   // ScenariosResponse
const readiness = await client.readiness();   // ReadinessResponse
const compete = await client.compete();       // CompeteResponse
const skill = await client.skill();           // SkillResponse
const signing = await client.signingPolicy(); // SigningPolicyResponse
```

The following methods now return precise result types (decimal fields are
serialized as `string` to avoid float drift, matching the Rust backend):
`regime()`, `funding()`, `scenarios()`, `readiness()`, `compete()`, `skill()`,
`signingPolicy()`, `proof()`, and `events()`.

## Proof verification

`src/proof.ts` is an independent, offline proof verifier that mirrors the Python
(`clients/proof-verifier/verify.py`) and Go (`clients/go/proof.go`) verifiers. It
recomputes the agent's cryptographic commitments from first principles and
validates the competition contract metadata — it never trusts the agent.

```ts
import { GuardrailClient, verifyProof, renderReport } from "@guardrail/client";

const client = new GuardrailClient({ baseUrl: "http://localhost:8080" });
const proof = await client.proof();

// Optionally pass the raw policy-file content to recompute policy_hash.
const result = await verifyProof(proof, { policyRaw });

console.log(renderReport(result));
if (!result.passed) {
  throw new Error("proof verification failed");
}
```

`verifyProof(proof, options?)` returns a typed `VerifyResult`:

```ts
interface VerifyResult {
  passed: boolean;                 // true iff no check FAILed (SKIPs are tolerated)
  checks: VerifyCheck[];           // { name, status: "PASS" | "FAIL" | "SKIP", detail }
}
```

Checks performed:

- `wallet_address` — `0x` + 40 hex (canonical) or the repo's vanity placeholder
- `policy_hash` — recomputed `sha256(policyRaw)` vs claimed (SKIP if no content)
- `report_hash` — recomputed `sha256` of the compact core object vs claimed
- `agent_id` — recomputed `sha256(name + 0x00 + wallet)` vs claimed
- `address_url` — must equal `https://bscscan.com/address/<wallet>`
- `registration_tx` — `0x` + 64 hex; URL must match BscScan when present
- `competition_contract_format` / `competition_contract_explorer_url`

SHA-256 is loaded through a guarded dynamic import of Node's `crypto`, falling
back to the Web Crypto `subtle` API. When neither backend is available, the
hash-dependent checks degrade to `SKIP` rather than throwing, so the module
still type-checks and bundles for the browser. The verifier is pure aside from
that one import.

## Example

A self-contained, offline verify + journal example lives in
`examples/verify-and-journal.ts`. It verifies the bundled sample proof and walks
an event journal through an in-memory `fetch` stub, exiting `0` on success:

```bash
cd clients/typescript && pnpm install && pnpm run example
```

## Methods

| Method | Route | Returns |
|---|---|---|
| `health()` | `/health` | API + DB status |
| `cockpit()` | `/cockpit` | aggregated live view |
| `portfolio()` | `/portfolio` | latest reconciliation |
| `trades()` | `/trades` | recent trades |
| `signals()` | `/signals` | latest signals |
| `risk()` | `/risk` | risk events + kill switch |
| `alerts()` | `/alerts` | evaluated alerts |
| `proof()` | `/proof` | agent identity + report proof |
| `events()` | `/events` | recent event log |
| `history()` | `/history` | NAV equity series |
| `readiness()` | `/readiness` | readiness probe |
| `exposure()` | `/exposure` | portfolio exposure |
| `briefing()` | `/briefing` | operator briefing |
| `budget()` | `/budget` | budget status |
| `heartbeat()` | `/heartbeat` | heartbeat status |
| `costs()` | `/costs` | cost accounting |
| `drift()` | `/drift` | allocation drift |
| `exitTriggers()` | `/exit-triggers` | exit triggers |
| `liquidity()` | `/liquidity` | liquidity view |
| `quotes()` | `/quotes` | latest quotes |
| `watchlist()` | `/watchlist` | watchlist |
| `rebalance()` | `/rebalance` | rebalance plan |
| `scenarios()` | `/scenarios` | stress scenarios |
| `metrics()` | `/metrics` | Prometheus text |
| `backtest(p)` | `/backtest` | strategy vs benchmark |
| `walkforward(p)` | `/walkforward` | rolling windows |
| `sweep(p)` | `/sweep` | sentiment comparison |
| `assets()` | `/assets` | tracked assets |
| `trending()` | `/trending` | trending assets |
| `regime()` | `/regime` | market regime |
| `funding()` | `/funding` | funding rates |
| `mandates()` | `/mandates` | mandate catalog |
| `experiments()` | `/experiments` | experiment log |
| `indicators(p)` | `/indicators` | synthetic indicators (`symbol`, `steps`) |
| `optimize(p)` | `/optimize` | basket weight optimization |
| `universe()` | `/universe` | trading universe |
| `config()` | `/config` | config inventory |
| `ops()` | `/ops` | ops status |
| `policy()` | `/policy` | active policy |
| `signingPolicy()` | `/signing-policy` | signing policy |
| `walletControls()` | `/wallet-controls` | wallet controls |
| `playbook()` | `/playbook` | operator playbook |
| `prizes()` | `/prizes` | prize catalog |
| `commerce()` | `/commerce` | commerce view |
| `sdkCatalog()` | `/sdk-catalog` | SDK catalog |
| `bnbSdk()` | `/bnb-sdk` | BNB SDK metadata |
| `report()` | `/report` | structured report JSON |
| `reportMarkdown()` | `/report/markdown` | Markdown report (text) |
| `exportSubmissionMarkdown()` | `/export/submission.md` | submission Markdown (text) |
| `scorecard()` | `/scorecard` | judge scorecard |
| `auditManifest()` | `/audit-manifest` | submission audit manifest |
| `skill()` | `/skill` | skill descriptor |
| `compete()` | `/compete` | competition status |
| `jobSimulator()` | `/job-simulator` | job simulator |
| `agentServices()` | `/agent-services` | agent services |
| `agentCard()` | `/agent-card` | agent card |
| `wellKnownAgentCard()` | `/.well-known/agent-card.json` | well-known agent card |
| `compilePolicy(m)` | `/policy/compile` | compiled policy + hash |

### Quant tools

| Method | Route | Description |
|---|---|---|
| `ta(p)` | `/ta` | technical indicator over a close-price series |
| `fees(p)` | `/fees` | all-in swap-cost estimate |
| `sizer(p)` | `/sizer` | position size by method (kelly / vol-target / fixed) |
| `orderbook(spec?)` | `/orderbook` | run the matching engine over an order spec |
| `pnl(fills?, marks?)` | `/pnl` | average-cost PnL attribution |
| `correlation(series?)` | `/correlation` | pairwise correlation matrix |
| `equityIndicators(ind?, period?)` | `/equity/indicators` | indicator over the live NAV curve |
| `portfolioRisk()` | `/portfolio/risk` | concentration metrics (HHI, effective-N) |
| `cmcCapabilities()` | `/cmc/capabilities` | CMC capability lineage descriptor |

## CLI (`guardrail`)

A dependency-free operator CLI ships in `src/cli.ts`, mirroring the Go
`guardrailctl`. It uses only Node built-ins plus the SDK client, and is
**offline-safe** (except `smoke`): every subcommand prints a notice and exits
`0` when the API is unreachable, so it is harmless in CI or against a stopped
backend. The lone exception is `smoke`, a deliberate pre-ship gate that exits
non-zero on failure.

Build it to `dist/` and run the compiled entrypoint:

```bash
cd clients/typescript
npm run build          # tsc -p tsconfig.build.json -> dist/
node dist/cli.js status
```

Or use the package scripts / bin:

```bash
npm run cli -- status            # build + run
node dist/cli.js help            # full usage
npx guardrail status             # via the "bin" entry once installed
```

### Subcommands

| Command | Routes | Description |
|---|---|---|
| `status` | `/regime` + `/compete` + `/readiness` | one-line status + readiness table |
| `regime` | `/regime` | current market regime + inputs |
| `journal` | `/journal` | compact per-cycle decision journal |
| `ensemble` | `/ensemble` | current regime + per-skill weight matrix |
| `skills [ID]` | `/skills`, `/skills/{id}` | Skill catalog, or one skill's detail |
| `verify` | `/proof/verify` | server-side proof pass/fail table |
| `snapshots` | `/snapshots` | latest run summary + per-asset prices |
| `watch` | `/regime` + `/compete` | refreshing one-line status, polled on an interval |
| `smoke` | all 9 quant endpoints | PASS/FAIL/WARN table; **non-zero exit on failure** |
| `help` | — | usage |

### Flags

- `--base URL` — API base URL. Defaults to `$GUARDRAIL_BASE_URL`, else
  `http://localhost:8080`. Accepts `--base URL` or `--base=URL`.
- `--json` — emit machine-readable JSON instead of the human-readable table.

```bash
node dist/cli.js status --json
node dist/cli.js skills momentum-v1 --base http://localhost:8080
GUARDRAIL_BASE_URL=http://host:8080 node dist/cli.js ensemble
```

### `watch`

`watch` polls `/regime` + `/compete` on an interval and rewrites a single
status line in place, so the terminal shows a live, refreshing view. It mirrors
the Go `guardrailctl watch`.

- `--interval N` — poll interval in seconds. Default `5`, clamped to a minimum
  of `1` to avoid busy-looping the API. Accepts `--interval N` or `--interval=N`.
- `--once` — print a single status tick and exit (handy for scripts/CI).
- `--json` — emit one discrete JSON object per tick (line-parseable stream)
  instead of the in-place status line.

It is offline-safe: unreachable endpoints render as `regime=offline` /
`compete=offline` (or `{ "status": "offline" }` in JSON) and the command still
exits `0`. Press `Ctrl-C` (SIGINT) to stop the loop cleanly.

```bash
node dist/cli.js watch                      # refresh every 5s until Ctrl-C
node dist/cli.js watch --interval 2         # refresh every 2s
node dist/cli.js watch --once               # single tick, then exit 0
node dist/cli.js watch --json --interval 10 # one JSON object every 10s
```

### `smoke`

`smoke` is the typed, cross-platform sibling of `scripts/smoke_quant.sh`: it
exercises every quant endpoint through the SDK and prints a `PASS`/`WARN`/`FAIL`
line per endpoint. A throw is `FAIL`, an `error` field in the response is `WARN`
(reachable but needs a prior agent run), otherwise `PASS`.

Unlike every other subcommand, `smoke` is a **gate**: it exits non-zero when any
endpoint fails to respond, so it is safe to wire into a pre-ship check.

```bash
node dist/cli.js smoke                       # against $GUARDRAIL_BASE_URL or :8080
node dist/cli.js smoke --base http://127.0.0.1:8091
node dist/cli.js smoke --json                # { base, fails, results: [...] }
```

The CLI is backed by these new typed SDK methods on `GuardrailClient`:
`ensemble()`, `journal()`, `snapshots(params?)`, `skills()`, `skillById(id)`,
and `proofVerify()` (return types in `src/cli-types.ts`).

## Type-check

```bash
cd clients/typescript && pnpm install && pnpm typecheck
```
