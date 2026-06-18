#!/usr/bin/env bash
# Render Cron Job entrypoint: restore the TWAK wallet keystore from a secret env
# var, then run one daily competition trade. The trader's own window guard
# (June 22–28) makes runs outside the window a safe no-op.
set -euo pipefail

if [ -z "${TWAK_ACCESS_ID:-}" ] || [ -z "${TWAK_WALLET_JSON_B64:-}" ]; then
  echo "ERROR: TWAK_ACCESS_ID / TWAK_WALLET_JSON_B64 env vars are required" >&2
  exit 2
fi

mkdir -p "$HOME/.twak"
echo "$TWAK_WALLET_JSON_B64" | base64 -d > "$HOME/.twak/wallet.json"
chmod 600 "$HOME/.twak/wallet.json"

exec python3 scripts/daily_trade.py --live
