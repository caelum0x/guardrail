# guardrail-client (Python)

Dependency-free Python client for the Guardrail Alpha read-only API.

Uses only the Python standard library (`urllib.request` + `json`) -- no
`requests` or `httpx`. The API is read-only; this client never mutates agent
state.

## Install

Python 3.10+ required. From the repo:

```bash
pip install ./clients/python
```

Or run directly from source by adding `clients/python` to your path.

## Usage

```python
from guardrail_client import GuardrailClient

client = GuardrailClient(base_url="http://localhost:8080", timeout=10.0)

# Live state
health = client.health()
alerts = client.alerts()
nav = client.history()

# Research
bt = client.backtest(steps=60, fear_greed=70, preset="balanced")
print(bt["metrics"]["total_return_pct"], bt.get("excess_return_pct"))

wf = client.walkforward(windows=6, steps=30, preset="aggressive")
sweep = client.sweep(steps=40, fear_greed=[20, 50, 80])

# Natural-language policy
compiled = client.compile_policy(
    "Trade CAKE and WBNB, max drawdown 20%, kill switch 25%, stable reserve 10%",
)
print(compiled.get("hash"), compiled.get("policy"))
```

## Offline proof verification

The package ships a dependency-free proof verifier (stdlib only: `hashlib` +
`json`) that independently re-derives the agent's cryptographic commitments and
compares them to the claimed values. It mirrors the Go (`clients/go/proof.go`),
TypeScript (`clients/typescript/src/proof.ts`), and standalone Python
(`clients/proof-verifier/verify.py`) ports, so all results agree byte-for-byte.

```python
import json
from guardrail_client import verify_proof, render_report

with open("clients/proof-verifier/sample_proof.json") as f:
    proof = json.load(f)

# policy_raw is optional: supply the exact bytes of the policy file to also
# recompute and check policy_hash. Omit it to SKIP that check.
result = verify_proof(proof)
print(render_report(result, source="sample_proof.json"))

if not result.passed:
    raise SystemExit(1)
```

`verify_proof` returns an immutable `VerifyResult`:

```python
@dataclass(frozen=True)
class VerifyResult:
    passed: bool            # True only when no check FAILED (skips do not fail)
    checks: tuple[Check]    # per-check results

@dataclass(frozen=True)
class Check:
    name: str               # e.g. "report_hash", "agent_id"
    status: str             # "PASS" | "FAIL" | "SKIP"
    detail: str
```

Commitments re-derived (mirroring the Rust agent):

- `agent_id    = sha256(name + "\x00" + wallet)`
- `report_hash = sha256(compact JSON of {run_id, cycles, final_nav_usd, total_drawdown_pct, events})`
- `policy_hash = sha256(raw policy-file bytes)` — checked only when `policy_raw` is supplied

It also validates the wallet / BscScan URL formats and the fixed competition
contract address + BscTrace explorer URL.

Verify a live agent in one call (fetches `/proof`, then verifies offline):

```python
from guardrail_client import GuardrailClient

client = GuardrailClient()
result = client.verify_proof()  # optionally pass policy_raw=<bytes>
print(result.passed)
```

A runnable, offline example lives at `examples/verify_proof.py` — it verifies
the bundled sample proof and exits non-zero if `report_hash` or `agent_id` do
not PASS:

```bash
python3 clients/python/examples/verify_proof.py
```

## Error handling

Non-2xx responses (and connection errors / timeouts) raise
`GuardrailApiError`:

```python
from guardrail_client import GuardrailClient, GuardrailApiError

client = GuardrailClient()
try:
    client.health()
except GuardrailApiError as exc:
    print(exc.status, exc.path, exc.body)
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
| `verify_proof(policy_raw=None)` | `/proof` | fetch + verify proof offline (`VerifyResult`) |
| `events()` | `/events` | recent event log |
| `history()` | `/history` | NAV equity series |
| `readiness()` | `/readiness` | readiness probe |
| `exposure()` | `/exposure` | portfolio exposure |
| `briefing()` | `/briefing` | operator briefing |
| `budget()` | `/budget` | budget status |
| `heartbeat()` | `/heartbeat` | heartbeat status |
| `costs()` | `/costs` | cost accounting |
| `drift()` | `/drift` | allocation drift |
| `exit_triggers()` | `/exit-triggers` | exit triggers |
| `liquidity()` | `/liquidity` | liquidity view |
| `quotes()` | `/quotes` | latest quotes |
| `watchlist()` | `/watchlist` | watchlist |
| `rebalance()` | `/rebalance` | rebalance plan |
| `scenarios()` | `/scenarios` | stress scenarios |
| `metrics()` | `/metrics` | Prometheus text (returns `str`) |
| `backtest(steps, fear_greed, preset)` | `/backtest` | strategy vs benchmark |
| `walkforward(windows, steps, preset)` | `/walkforward` | rolling windows |
| `sweep(steps, fear_greed, preset)` | `/sweep` | sentiment comparison |
| `assets()` | `/assets` | tracked assets |
| `trending()` | `/trending` | trending assets |
| `regime()` | `/regime` | market regime |
| `funding()` | `/funding` | funding rates |
| `mandates()` | `/mandates` | mandate catalog |
| `experiments()` | `/experiments` | experiment log |
| `indicators(symbol, steps)` | `/indicators` | synthetic indicators |
| `optimize(symbols, scores, vols)` | `/optimize` | basket weight optimization |
| `universe()` | `/universe` | trading universe |
| `config()` | `/config` | config inventory |
| `ops()` | `/ops` | ops status |
| `policy()` | `/policy` | active policy |
| `signing_policy()` | `/signing-policy` | signing policy |
| `wallet_controls()` | `/wallet-controls` | wallet controls |
| `playbook()` | `/playbook` | operator playbook |
| `prizes()` | `/prizes` | prize catalog |
| `commerce()` | `/commerce` | commerce view |
| `sdk_catalog()` | `/sdk-catalog` | SDK catalog |
| `bnb_sdk()` | `/bnb-sdk` | BNB SDK metadata |
| `report()` | `/report` | structured report JSON |
| `report_markdown()` | `/report/markdown` | Markdown report (returns `str`) |
| `export_submission_markdown()` | `/export/submission.md` | submission Markdown (returns `str`) |
| `scorecard()` | `/scorecard` | judge scorecard |
| `audit_manifest()` | `/audit-manifest` | submission audit manifest |
| `skill()` | `/skill` | skill descriptor |
| `compete()` | `/compete` | competition status |
| `job_simulator()` | `/job-simulator` | job simulator |
| `agent_services()` | `/agent-services` | agent services |
| `agent_card()` | `/agent-card` | agent card |
| `well_known_agent_card()` | `/.well-known/agent-card.json` | well-known agent card |
| `compile_policy(mandate)` | `/policy/compile` | compiled policy + hash |

All JSON methods return parsed `dict` objects; `metrics()` returns text.

## Quickstart

A runnable example lives at `examples/quickstart.py`:

```bash
python3 clients/python/examples/quickstart.py
```

It prints `health()` and `backtest()` output, and degrades gracefully with a
notice when the API is unreachable.
