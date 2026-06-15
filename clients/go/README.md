# Guardrail Go SDK

A dependency-free, idiomatic Go client for the Guardrail Alpha read/control
HTTP API. It uses only the standard library (`net/http`, `encoding/json`,
`context`) and mirrors the route set and JSON shapes of the sibling TypeScript
and Python SDKs.

The API is **read-only** — this client never mutates agent state.

## Install

```bash
go get github.com/guardrail-alpha/guardrail-go
```

Requires Go 1.24+.

## Import

```go
import guardrail "github.com/guardrail-alpha/guardrail-go"
```

## Usage

```go
package main

import (
	"context"
	"log"
	"time"

	guardrail "github.com/guardrail-alpha/guardrail-go"
)

func main() {
	client := guardrail.NewClient("http://localhost:8080",
		guardrail.WithTimeout(5*time.Second),
	)

	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()

	health, err := client.Health(ctx)
	if err != nil {
		log.Fatal(err)
	}
	log.Printf("ok=%t events_visible=%d", health.OK, health.EventsVisible)

	bt, err := client.Backtest(ctx, guardrail.BacktestParams{
		Steps:     60,
		FearGreed: 70,
		Preset:    guardrail.PresetBalanced,
	})
	if err != nil {
		log.Fatal(err)
	}
	log.Printf("total return: %s", bt.Metrics.TotalReturnPct)
}
```

## Options

`NewClient(baseURL string, opts ...Option)` accepts functional options:

- `WithHTTPClient(*http.Client)` — supply a custom HTTP client (proxies,
  transports, tests).
- `WithTimeout(time.Duration)` — set the request timeout on the underlying
  HTTP client. Per-call `context` deadlines also apply.

If `baseURL` is empty, `guardrail.DefaultBaseURL` (`http://localhost:8080`) is
used.

## Error handling

Every call takes a `context.Context` as its first argument. On a non-2xx
response the client returns an `*APIError` carrying the HTTP status and the raw
body:

```go
var apiErr *guardrail.APIError
if errors.As(err, &apiErr) {
	log.Printf("status %d: %s", apiErr.Status, apiErr.Body)
}
```

Transport failures (such as a refused connection) are returned as ordinary
wrapped errors.

## Typed vs dynamic payloads

Headline routes return first-class structs:

| Method | Route | Returns |
| --- | --- | --- |
| `Health` | `/health` | `*HealthResponse` |
| `Proof` | `/proof` | `*Proof` |
| `Compete` | `/compete` | `*CompeteResponse` |
| `History` | `/history` | `*HistoryResponse` |
| `Regime` | `/regime` | `*RegimeResponse` |
| `Alerts` | `/alerts` | `*AlertsResponse` |
| `Backtest` | `/backtest` | `*BacktestResponse` |
| `WalkForward` | `/walkforward` | `*WalkForwardResponse` |
| `Sweep` | `/sweep` | `*SweepResponse` |
| `Events` | `/events` | `*EventsResponse` |
| `CompilePolicy` | `/policy/compile` | `*CompiledPolicyResponse` |

Dynamic endpoints (for example `Cockpit`, `Assets`, `Trending`, `Indicators`,
`Optimize`, `Funding`, `Skill`, `Report`) return `map[string]any` to stay
forward-compatible with backend changes. Text endpoints (`Metrics`,
`ReportMarkdown`, `ExportSubmissionMarkdown`) return `string`.

## Example

A runnable quickstart lives in [`example/`](./example). It calls several
endpoints with a short context timeout and exits cleanly even when the API is
offline:

```bash
cd example && go run .
```

## Proof verification

The SDK ships an independent, **offline** proof verifier — a Go port of the
`clients/proof-verifier` Python tool that shares no code with the Rust agent. It
re-derives every commitment in a `/proof` document from first principles using
only `crypto/sha256` and compares them to the claimed values, so a third party
can confirm the agent's identity is genuinely reproducible rather than merely
asserted.

Fetch and verify a proof, or verify one loaded from disk:

```go
// From a running API:
proof, err := client.Proof(ctx) // *guardrail.Proof
if err == nil {
	result := proof.Verify("/proof", "configs/risk_policy.paper.json")
	fmt.Println(result.Report())
	if !result.Passed {
		// at least one check FAILED
	}
}

// From a file (no network):
proof, _ := guardrail.LoadProofFile("sample_proof.json")
result := proof.Verify("sample_proof.json", "") // "" skips the policy_hash file check
```

`Verify` returns a typed `VerifyResult{Passed, Checks []Check}`. Each `Check`
has a `Name`, a `Status` (`PASS`, `FAIL`, or `SKIP`), and a human-readable
`Detail`. Commitments a given proof shape does not carry (for example a bare
`run_report.json` omits `report_hash` and `agent_id`) are reported as `SKIP`
rather than failing. `result.Report()` renders the full text report and
`result.Counts()` returns the pass/fail/skip tallies.

What is checked: `wallet_address` format, `policy_hash` (recomputed
`sha256` of the policy file when one is supplied), `report_hash` (recomputed
`sha256` of the compact `{run_id, cycles, final_nav_usd, total_drawdown_pct,
events}` core object), `agent_id` (`sha256(name + 0x00 + wallet)`),
`address_url`, `registration_tx`, and the fixed competition contract address +
BscScan/BscTrace explorer URL formats.

### Verifier command

A runnable verifier lives in [`example/verify/`](./example/verify). It loads a
proof from a file argument (defaulting to the bundled
`clients/proof-verifier/sample_proof.json` fixture) or fetches `/proof` from a
running API, prints the PASS/FAIL report, and exits 0 even when the file or API
is unavailable:

```bash
# Verify the bundled offline fixture (default).
go run ./example/verify

# Verify a specific proof document.
go run ./example/verify ../proof-verifier/sample_proof.json

# Recompute policy_hash against an explicit policy file.
go run ./example/verify -policy-file ../../configs/risk_policy.paper.json ../proof-verifier/sample_proof.json

# Fetch and verify the live /proof envelope.
go run ./example/verify -url http://localhost:8080
```
