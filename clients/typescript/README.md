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

## Type-check

```bash
cd clients/typescript && pnpm install && pnpm typecheck
```
