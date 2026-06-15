#!/usr/bin/env python3
"""Guardrail control bot — a READ-ONLY chat window onto the running agent.

It answers a fixed set of commands (/status /regime /journal /verify /skills)
by querying the read-only Guardrail API and posting formatted replies to
Telegram and/or Discord. It NEVER signs, trades, or mutates state — self-custody
and trade authority stay entirely with the Rust engine + TWAK.

Secrets (bot tokens, webhook URLs) come ONLY from environment variables, never
the config file or CLI. ``--dry-run`` (the DEFAULT) prints what would be sent and
makes no outbound chat calls, so it runs fully offline.

Usage:
    python3 services/control-bot/bot.py --once --dry-run
    python3 services/control-bot/bot.py --live            # long-poll Telegram
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.request

import commands as cmd

DEFAULT_API = "http://localhost:8080"
DEFAULT_POLL_SECONDS = 5.0


def _post(url: str, payload: dict, dry_run: bool) -> None:
    """POST JSON to a chat sink, or print it in dry-run."""
    if dry_run:
        print(f"[dry-run] -> {url}\n         {json.dumps(payload)}")
        return
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        url, data=data, headers={"Content-Type": "application/json"}
    )
    try:
        urllib.request.urlopen(req, timeout=10)  # noqa: S310
    except (urllib.error.URLError, OSError, TimeoutError) as exc:
        print(f"sink error ({url}): {exc}", file=sys.stderr)


def deliver(text: str, dry_run: bool) -> None:
    """Send a reply to whichever sinks are configured via env vars."""
    tg_token = os.environ.get("GUARDRAIL_TELEGRAM_BOT_TOKEN")
    tg_chat = os.environ.get("GUARDRAIL_TELEGRAM_CHAT_ID")
    discord = os.environ.get("GUARDRAIL_DISCORD_WEBHOOK_URL")

    if tg_token and tg_chat:
        _post(
            f"https://api.telegram.org/bot{tg_token}/sendMessage",
            {"chat_id": tg_chat, "text": text, "parse_mode": "Markdown"},
            dry_run,
        )
    if discord:
        _post(discord, {"content": text}, dry_run)
    if not (tg_token and tg_chat) and not discord:
        # No sink configured: emit to stdout so the bot is still useful offline.
        print(text)


def run_once(base_url: str, dry_run: bool) -> int:
    """Answer every command once (useful for testing / a status digest)."""
    for command in cmd.COMMANDS:
        deliver(cmd.answer(base_url, command), dry_run)
    return 0


def run_loop(base_url: str, dry_run: bool, poll_seconds: float) -> int:
    """Long-poll Telegram getUpdates for commands and answer them.

    In dry-run (default) there is no chat connection, so this degrades to a
    periodic status digest instead of polling — keeping it offline-safe.
    """
    tg_token = os.environ.get("GUARDRAIL_TELEGRAM_BOT_TOKEN")
    if dry_run or not tg_token:
        print("(dry-run / no Telegram token: emitting a status digest each tick; "
              "Ctrl-C to stop)")
        try:
            while True:
                deliver(cmd.answer(base_url, "status"), dry_run=True)
                time.sleep(poll_seconds)
        except KeyboardInterrupt:
            return 0

    offset = 0
    while True:
        try:
            url = (
                f"https://api.telegram.org/bot{tg_token}/getUpdates"
                f"?timeout=30&offset={offset}"
            )
            with urllib.request.urlopen(url, timeout=40) as resp:  # noqa: S310
                updates = json.loads(resp.read().decode("utf-8")).get("result", [])
            for upd in updates:
                offset = upd["update_id"] + 1
                text = (upd.get("message") or {}).get("text", "")
                if text.startswith("/"):
                    deliver(cmd.answer(base_url, text.split()[0]), dry_run=False)
        except (urllib.error.URLError, OSError, ValueError, TimeoutError) as exc:
            print(f"poll error: {exc}", file=sys.stderr)
            time.sleep(poll_seconds)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Guardrail read-only control bot.")
    parser.add_argument("--api", default=os.environ.get("GUARDRAIL_API", DEFAULT_API))
    parser.add_argument("--once", action="store_true", help="answer all commands once and exit")
    parser.add_argument("--poll-seconds", type=float, default=DEFAULT_POLL_SECONDS)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--dry-run", dest="dry_run", action="store_true", default=True)
    mode.add_argument("--live", dest="dry_run", action="store_false")
    args = parser.parse_args(argv)

    if args.once:
        return run_once(args.api, args.dry_run)
    return run_loop(args.api, args.dry_run, args.poll_seconds)


if __name__ == "__main__":
    raise SystemExit(main())
