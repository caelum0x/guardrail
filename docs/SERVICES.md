# Companion Services (`services/`)

Out-of-process, **read-only** services that consume the read-only Guardrail API
or the analytics layer. None sign, trade, or hold keys — trade authority stays
with the Rust engine + TWAK. All are standard-library Python and offline-safe
(dry-run / `--check` defaults need no network or secrets). Secrets come only from
environment variables.

| Service | What it does | Offline-safe entry point |
|---|---|---|
| `services/control-bot/` | Read-only Telegram/Discord bot: `/status` `/regime` `/journal` `/verify` `/skills` over the API | `python3 services/control-bot/bot.py --once --dry-run` |
| `services/gateway/` | Stdlib edge proxy in front of the API: per-IP rate limit + CORS + short-TTL GET cache; only GET/HEAD/OPTIONS proxied | `python3 services/gateway/gateway.py --check` |
| `services/report-publisher/` | Renders the python-lab HTML report bundle (dossier/journal/ensemble) to a published dir | `python3 services/report-publisher/publisher.py --dry-run` |

These complement [`integrations/alert-relay`](../integrations/alert-relay) (alert
forwarding with Telegram/Discord/Slack/email/webhook sinks). See
[`services/README.md`](../services/README.md).

## Safety model
- Read-only: the bot and gateway only issue GETs to the read-only API; the
  gateway returns `405` for writes.
- No secrets in source: tokens/webhooks are env-var references only.
- Graceful degradation: every service exits 0 and prints a clear notice when the
  API/upstream is down.
