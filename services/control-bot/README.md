# Guardrail Control Bot (read-only)

A read-only chat window onto the running agent. It answers a fixed set of
commands by querying the **read-only** Guardrail API and posting formatted
replies to Telegram and/or Discord. It **never signs, trades, or mutates state** —
trade authority stays with the Rust engine + TWAK.

## Commands
`/status` · `/regime` · `/journal` · `/verify` · `/skills`

## Run
```bash
# offline-safe digest (default): prints what it would send, no chat calls
python3 services/control-bot/bot.py --once --dry-run

# live Telegram long-poll (requires env tokens)
python3 services/control-bot/bot.py --live
```

## Configuration (env only — never commit secrets)
| Variable | Purpose |
|---|---|
| `GUARDRAIL_API` | API base URL (default `http://localhost:8080`) |
| `GUARDRAIL_TELEGRAM_BOT_TOKEN` | Telegram bot token (live mode) |
| `GUARDRAIL_TELEGRAM_CHAT_ID` | Telegram chat to post to |
| `GUARDRAIL_DISCORD_WEBHOOK_URL` | Discord webhook (optional) |

With no sink configured the bot prints replies to stdout, so it is useful even
fully offline. Standard library only — no dependencies.
