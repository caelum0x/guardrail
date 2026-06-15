# ADR 0009 — Alert relay as an out-of-process, read-only API consumer

- Status: Accepted
- Date: 2026-06-14

## Context

Operators need alerts forwarded to human channels (Telegram, Discord, generic
webhook), but the agent that holds custody and writes the event log (ADR 0004)
must stay self-contained and minimal. Embedding chat clients and delivery
secrets inside the trading process would widen its attack surface, couple
notification uptime to trading correctness, and put third-party dependencies
next to the signing path.

## Decision

Run notifications as a **separate, out-of-process, read-only consumer**:
`integrations/alert-relay/` (`relay.py` + `sinks.py`), standard-library only.
The Guardrail API evaluates typed alert conditions as pure functions over the
run report and recent event log and exposes them at `GET /alerts` (and
`GET /readiness`). The relay only performs HTTP GETs against that read-only API:
it imports no Rust crates, holds no trading keys, and cannot mutate any
Guardrail state. It polls, filters by a configurable severity threshold,
deduplicates by hashing `kind + severity + message`, and dispatches.

It is **offline-safe by default**: `--dry-run` is the default and opens no sink
connection; a down/unreachable API is logged and never crashes the loop. Only
delivery secrets (chat tokens / webhook URLs) live in the relay, read from env
at runtime — never in code or committed config.

## Consequences

- The relay can be deployed, restarted, or removed with zero impact on the
  agent's correctness or custody guarantees; the trading process never gains a
  notification dependency.
- Smoke test is one command:
  `python3 integrations/alert-relay/relay.py --once --dry-run` (exits 0 even
  with the API down). Live delivery requires `--live`, enabled sinks, and
  secrets in env.
- Severity routing is per-instance; to fan different severities to different
  channels, run multiple relays with different configs/thresholds.
- Trade-off: at-least-once delivery — restarting the relay resets dedup memory,
  so an alert may re-fire once, the safe default for a notifier.
- See `docs/ALERTING.md` and `integrations/alert-relay/README.md` for the
  config schema and env vars.
