# guardrail-term — Guardrail terminal client (TypeScript)

A typed, **dependency-free** Node CLI for the read-only Guardrail API. Uses
Node's built-in `fetch` (Node 18+) and prints colored, aligned tables (or raw
JSON). No runtime packages; builds with just TypeScript.

## Build / run

```bash
cd clients/ts-terminal
npx -y typescript --project tsconfig.json   # or: npm run build
node dist/index.js status
GUARDRAIL_API=http://127.0.0.1:8080 node dist/index.js regime
node dist/index.js --json portfolio
```

## Commands

`status regime portfolio trades risk signals proof verify events cockpit watch`

```bash
guardrail-term status            # GET /health
guardrail-term verify            # GET /proof/verify (colored pass/fail table)
guardrail-term events            # GET /events (last 15)
guardrail-term --json risk       # raw JSON for any command
guardrail-term watch 10          # poll /regime every 10s
```

## Flags

- `--api=URL` — API base (default `$GUARDRAIL_API` or `http://127.0.0.1:8080`)
- `--json` — raw JSON instead of a table

Read-only by construction: the client only issues `GET`s.
