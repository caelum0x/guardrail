# Alerting

This document describes how Guardrail surfaces operator alerts and how the
**alert relay** forwards them to human channels (Telegram, Discord, generic
webhook).

## Where alerts come from

The Guardrail API evaluates a fixed set of typed alert conditions on every
request to `/alerts`. The evaluators are pure functions over a run report and
the recent event log (see `crates/observability/src/alerts.rs`). The conditions
are:

| Kind | Severity (typical) | Meaning |
|------|--------------------|---------|
| `drawdown_soft` | warning | Drawdown crossed the soft warning limit. |
| `drawdown_hard` | critical | Drawdown crossed the hard stop limit. |
| `data_stale` | critical | Market data older than the freshness budget. |
| `slippage_high` | warning | Realized slippage exceeded tolerance. |
| `recon_mismatch` | critical | Internal vs. broker reconciliation disagrees. |
| `kill_switch` | critical | The kill switch is engaged. |
| `daily_trade_missing` | warning | An expected daily trade did not occur. |

The `/alerts` endpoint returns an ordered list (most severe first) plus a
rolled-up `status` (`clear` / `warning` / `critical`) and `counts`. The related
`/readiness` endpoint reports go/no-go checks, including a "No critical operator
alerts" check, so alerting and readiness share the same underlying signal.

## How the relay fits the stack

```
+------------------+        GET /alerts          +-------------------+
|  Guardrail API   |  <------------------------   |   alert-relay     |
| (read-only;      |   GET /readiness (optional)  |   (this tool)     |
|  owns all state) |  ------------------------->   |  poll + dedup     |
+------------------+        JSON response          +---------+---------+
                                                             |
                                          dispatch new alerts (>= threshold)
                                                             |
                          +----------------+----------------+----------------+
                          v                v                                 v
                     Telegram          Discord                       Generic webhook
```

Key boundary properties:

- **The agent/API stays self-contained.** The relay is a *consumer only*: it
  performs HTTP GETs against the read-only API. It imports no Rust crates,
  holds no trading keys, and cannot mutate any Guardrail state.
- **The relay holds only delivery secrets.** Chat tokens/webhook URLs are read
  from environment variables at runtime, never from code or committed config.
- **Offline-safe by default.** The relay's default mode is `--dry-run`, which
  never opens a connection to a sink. If the API itself is unreachable, the
  relay logs `API unreachable / no alerts` and continues without crashing.

This separation means the relay can be deployed, restarted, or removed without
any impact on the agent's correctness or custody guarantees.

## Severity routing

The relay filters by a configurable `severity_threshold`
(`info < warning < critical`). Only alerts at or above the threshold are
forwarded; the rest are logged for the record. Recommended routing:

| Environment | Threshold | Rationale |
|-------------|-----------|-----------|
| Local / paper | `info` | See everything while testing. |
| Staging | `warning` | Filter noise, still catch soft limits. |
| Production | `warning` or `critical` | Page only on actionable conditions. |

All enabled sinks currently receive every alert that clears the threshold. To
route different severities to different destinations, run multiple relay
instances with different configs and thresholds (for example, a `critical`-only
relay pointed at an on-call channel plus a `warning` relay to a team channel).

## Deduplication

Alerts have no server-assigned id, so the relay derives a stable id by hashing
`kind + severity + message`. Within a single relay process, each unique alert is
delivered exactly once; re-observing the same alert on subsequent polls is a
no-op. Restarting the relay resets the dedup memory (alerts may re-fire once),
which is the safe default for an at-least-once notifier.

## Operational notes

- **Run modes.** `--once` does a single poll (good for cron probes / CI);
  omitting it runs a continuous loop at `poll_interval_seconds`.
- **Failure isolation.** A down API, malformed response, or failing sink is
  logged and never propagates out of the loop.
- **Exit codes.** `--once --dry-run` exits `0` even with the API down (safe
  smoke test). Only fatal config errors exit non-zero (`2`).
- **No third-party deps.** Pure Python stdlib; deploy by copying two files.
- **Readiness probe.** With `include_readiness: true`, each poll also logs the
  `/readiness` status and blocking-check count for at-a-glance health.

See `integrations/alert-relay/README.md` for setup, the config schema, and the
env vars used for secrets.
