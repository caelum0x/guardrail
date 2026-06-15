# Guardrail Alert Relay

A standalone, **standard-library-only** Python relay that polls the Guardrail
API and forwards operator alerts to chat and email sinks: **Telegram**,
**Discord**, **Slack**, a **generic webhook**, and **email (SMTP)**.

It has **zero third-party dependencies** (only `urllib`/`json` from the Python
stdlib) and is **offline-safe by default**: `--dry-run` is the default mode and
makes no network calls to sinks. A down API never crashes the relay.

The relay is a *consumer* of the read-only Guardrail API. It does not import
any Rust crates, hold any keys, or modify any state. It only reads `/alerts`
(and optionally `/readiness`) and forwards what it sees.

## Files

| File | Purpose |
|------|---------|
| `relay.py` | Poll loop, dedup, threshold filtering, dispatch, CLI. |
| `sinks.py` | One class per sink (Telegram / Discord / Slack / webhook / email), each with a dry-run path. |
| `Dockerfile` | Small `python:3-slim` image that runs `relay.py --live`, configured via env + mounted config. |
| `run.sh` | Offline-safe wrapper; defaults to `--once --dry-run`. |
| `../../configs/alerts.example.json` | Example config with placeholder, env-referenced secrets and `enabled: false`. |

## Requirements

- Python 3.8+ (standard library only; nothing to `pip install`).

## Quick start

```bash
# Offline smoke test: single poll, dry-run (DEFAULT). Safe even with the API down.
python3 integrations/alert-relay/relay.py --once --dry-run

# Continuous dry-run loop (prints what WOULD be sent).
python3 integrations/alert-relay/relay.py

# Live single poll (real delivery; requires secrets in env + enabled sinks).
python3 integrations/alert-relay/relay.py --once --live

# Use a custom config file.
python3 integrations/alert-relay/relay.py --config configs/alerts.json
```

By default the relay loads `configs/alerts.example.json` (resolved relative to
the repo root), so it runs from any working directory.

## CLI flags

| Flag | Default | Meaning |
|------|---------|---------|
| `--config PATH` | `configs/alerts.example.json` | Config file to load. |
| `--once` | off | Run a single poll cycle, then exit. |
| `--dry-run` | **on** | Print what would be sent; no sink network calls. |
| `--live` | off | Actually deliver to sinks (mutually exclusive with `--dry-run`). |
| `--interval SECS` | from config | Override the poll interval. |

## Secrets (never hardcoded)

Sink credentials are **never** stored in the config or the code. Each
secret-bearing field in the config uses an `env:VAR_NAME` reference, and the
relay reads the real value from the environment at runtime:

```bash
export GUARDRAIL_TELEGRAM_BOT_TOKEN="123456:your-bot-token"
export GUARDRAIL_TELEGRAM_CHAT_ID="-1001234567890"
export GUARDRAIL_DISCORD_WEBHOOK_URL="https://discord.com/api/webhooks/.../..."
export GUARDRAIL_GENERIC_WEBHOOK_URL="https://example.com/hooks/guardrail"
export GUARDRAIL_SLACK_WEBHOOK_URL="https://hooks.example.invalid/services/.../..."
export GUARDRAIL_SMTP_HOST="smtp.example.com"
export GUARDRAIL_SMTP_SENDER="guardrail@example.com"
export GUARDRAIL_SMTP_RECIPIENTS="oncall@example.com,ops@example.com"
export GUARDRAIL_SMTP_USERNAME="guardrail@example.com"
export GUARDRAIL_SMTP_PASSWORD="your-smtp-password"
```

> Note: the Slack webhook URL embeds its own secret token, so the *whole URL*
> is treated as a secret and supplied via `env:`. The same applies to the
> Discord and generic webhook URLs.

If an env var is missing, it resolves to an empty string. In `--dry-run` no
secrets are needed at all; in `--live` a sink with a missing secret reports
itself as misconfigured and is skipped (the relay keeps running).

## Config schema

```jsonc
{
  "poll_interval_seconds": 60,        // loop interval (ignored with --once)
  "sink_timeout_seconds": 10,         // per-request network timeout
  "severity_threshold": "warning",    // "info" | "warning" | "critical"
  "include_readiness": true,          // also probe /readiness each poll
  "api": {
    "base_url": "http://127.0.0.1:8080"
  },
  "sinks": [
    {
      "kind": "telegram",             // telegram | discord | slack | webhook | email
      "enabled": false,               // disabled sinks are skipped
      "token": "env:GUARDRAIL_TELEGRAM_BOT_TOKEN",
      "chat_id": "env:GUARDRAIL_TELEGRAM_CHAT_ID",
      "api_base": "https://api.telegram.org"
    },
    {
      "kind": "discord",
      "enabled": false,
      "webhook_url": "env:GUARDRAIL_DISCORD_WEBHOOK_URL"
    },
    {
      "kind": "webhook",
      "enabled": false,
      "url": "env:GUARDRAIL_GENERIC_WEBHOOK_URL"
    },
    {
      "kind": "slack",                // Slack incoming-webhook
      "enabled": false,
      "webhook_url": "env:GUARDRAIL_SLACK_WEBHOOK_URL"
    },
    {
      "kind": "email",                // SMTP via the Python stdlib
      "enabled": false,
      "host": "env:GUARDRAIL_SMTP_HOST",
      "port": 587,                    // 587 = STARTTLS submission (default)
      "use_tls": true,                // STARTTLS when the server supports it
      "sender": "env:GUARDRAIL_SMTP_SENDER",
      "recipients": "env:GUARDRAIL_SMTP_RECIPIENTS", // comma/array of addresses
      "username": "env:GUARDRAIL_SMTP_USERNAME",
      "password": "env:GUARDRAIL_SMTP_PASSWORD",
      "subject_prefix": "[Guardrail]"
    }
  ]
}
```

### Sink kinds

| Kind | Transport | Required fields |
|------|-----------|-----------------|
| `telegram` | Telegram Bot API `sendMessage` | `token`, `chat_id` |
| `discord` | Discord incoming webhook | `webhook_url` |
| `slack` | Slack incoming webhook (`{"text": ...}`) | `webhook_url` |
| `webhook` | Generic HTTP `POST` of the structured alert | `url` |
| `email` | SMTP via stdlib `smtplib` / `email` | `host`, `sender`, `recipients` |

The **email** sink composes a standard RFC 5322 message and, in `--dry-run`,
prints the entire composed message (headers + body) without opening any SMTP
connection. In `--live` it connects to `host:port`, upgrades with STARTTLS when
`use_tls` is set, authenticates if `username`/`password` are present, and sends.
Every SMTP and network error is caught and logged, never crashing the loop.
`recipients` accepts either a JSON array or a comma/semicolon-separated string.

### Severity threshold

Only alerts at or above `severity_threshold` are dispatched. Ordering is
`info < warning < critical`. For example, with `"warning"` set, `info` alerts
are logged but not forwarded.

## How it consumes the API

The relay reads the Guardrail API's `/alerts` response:

```json
{
  "status": "clear|warning|critical",
  "counts": { "critical": 0, "warning": 1, "total": 1 },
  "alerts": [
    { "kind": "drawdown_soft", "severity": "warning", "message": "..." }
  ],
  "inputs": { "...": "..." }
}
```

Each alert has no explicit id, so the relay derives a stable dedup id by
hashing `kind + severity + message` (SHA-256, truncated). An alert is delivered
exactly once per process run; identical alerts on later polls are skipped.

When `include_readiness` is true, the relay also probes `/readiness` and logs
the overall status and the number of blocking checks for operator visibility.

## Dry-run vs live

- **dry-run (default):** No connection to Telegram/Discord/your webhook. The
  relay prints the exact message body it *would* send for each sink. The API
  poll itself is a normal read; if the API is down it prints
  `API unreachable / no alerts` and exits cleanly.
- **live (`--live`):** Real `POST`s to each enabled sink. Requires the relevant
  env vars to be set. Any sink failure is logged and never aborts the loop.

## Container & wrapper script

### `run.sh` (offline-safe wrapper)

`run.sh` is the easiest way to invoke the relay. With no arguments it runs a
single **dry-run** poll, so it is safe to run anywhere (no secrets, no sink
network calls):

```bash
# Single offline dry-run poll (DEFAULT). Always exits 0 offline.
./integrations/alert-relay/run.sh

# Any args you pass REPLACE the defaults, so you control dry-run vs live:
./integrations/alert-relay/run.sh --live
./integrations/alert-relay/run.sh --live --config configs/alerts.json

# Or drive it via env (handy inside containers / CI):
RELAY_ARGS="--live" ./integrations/alert-relay/run.sh
PYTHON_BIN=python3.12 ./integrations/alert-relay/run.sh
```

### Docker

The `Dockerfile` builds a small `python:3-slim` image with **no pip install**
(the relay is stdlib-only). It runs as a non-root user and contains **no
secrets** — credentials are passed at run time as env vars that the config
references via `env:VAR_NAME`.

```bash
# Build.
docker build -t guardrail-alert-relay -f integrations/alert-relay/Dockerfile .

# Offline smoke test (single dry-run poll), overriding the default command:
docker run --rm guardrail-alert-relay python3 relay.py --once --dry-run

# Continuous LIVE loop (default command) with a mounted config + secrets:
docker run --rm \
  -e GUARDRAIL_SLACK_WEBHOOK_URL="https://hooks.example.invalid/services/..." \
  -e GUARDRAIL_SMTP_HOST="smtp.example.com" \
  -e GUARDRAIL_SMTP_SENDER="guardrail@example.com" \
  -e GUARDRAIL_SMTP_RECIPIENTS="oncall@example.com" \
  -v "$PWD/configs/alerts.json:/app/configs/alerts.example.json:ro" \
  guardrail-alert-relay
```

The default `CMD` is `--live --config /app/configs/alerts.example.json`. Mount
your real config over that path, or override the command (as above) for a
dry-run probe.

## Operational notes

- The relay never raises on a down API, malformed JSON, or a failing sink.
- `--once --dry-run` always exits `0` (suitable for CI smoke tests / cron probes).
- Fatal config errors (missing file, invalid JSON, missing `api.base_url`)
  exit with code `2` so an operator notices immediately.
- See `docs/ALERTING.md` for how the relay fits into the wider stack.
