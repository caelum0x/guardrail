# grctl — Guardrail operator CLI (Go)

A small, dependency-free **read-only** Go CLI for the Guardrail API. It reads
the same endpoints the dashboard does and prints clean tables (or raw JSON).

## Build / run

```bash
cd clients/go-cli
go build -o grctl .
./grctl status
GUARDRAIL_API=http://127.0.0.1:8080 ./grctl regime
go run . --json portfolio
```

## Commands

`status regime portfolio trades risk signals proof verify events cockpit watch`

```bash
grctl status                 # GET /health
grctl regime                 # GET /regime
grctl verify                 # GET /proof/verify (pass/fail + check count)
grctl events                 # GET /events (last 10, timestamp + type)
grctl --json risk            # raw JSON for any command
grctl watch 10               # poll /regime every 10 seconds
```

## Flags

- `--api=URL` — API base (default `$GUARDRAIL_API` or `http://127.0.0.1:8080`)
- `--json` — print raw indented JSON instead of a summary table

Read-only by construction: the client only issues `GET`s and has no path to the
trading loop.
