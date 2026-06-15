# Guardrail Services

Out-of-process, **read-only** companion services. None of them sign, trade, or
hold keys — they consume the read-only Guardrail API (or the analytics layer) and
present/forward results. All are standard-library Python and offline-safe
(dry-run / `--check` defaults require no network or secrets).

| Service | What it does | Safe entry point |
|---|---|---|
| [`control-bot/`](./control-bot) | Read-only Telegram/Discord bot answering `/status` `/regime` `/journal` `/verify` `/skills` | `bot.py --once --dry-run` |
| [`gateway/`](./gateway) | Stdlib edge proxy: per-IP rate limit + CORS + GET caching in front of the API | `gateway.py --check` |
| [`report-publisher/`](./report-publisher) | Renders the HTML report bundle to a published dir | `publisher.py --dry-run` |

These complement the in-repo [`integrations/alert-relay`](../integrations/alert-relay)
(alert forwarding) and follow the same conventions: secrets via env only, network
guarded, graceful when the API is down. See [`docs/SERVICES.md`](../docs/SERVICES.md).
