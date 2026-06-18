# Render Cron Job image for the Track-1 daily trader.
# node (for @trustwallet/cli) + python3 (for the stdlib-only trader). The wallet
# keystore is restored from the TWAK_WALLET_JSON_B64 env var at runtime, so no
# secret ever lives in the image or the repo.
FROM node:22-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && npm install -g @trustwallet/cli

WORKDIR /app
COPY scripts/daily_trade.py scripts/daily_trade.py
COPY deploy/trader/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
