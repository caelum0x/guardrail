# price-oracle

A small **read-only** Go microservice that serves live USD prices for the
Guardrail BSC universe, sourced from the free [CoinGecko public
API](https://www.coingecko.com/en/api) and cached behind a TTL. No API key, no
chain access, no writes — just fast, cached market prices for the dashboard and
other read-only consumers.

## Run

```bash
cd services/price-oracle
go run .                 # listens on :8090
PORT=9000 ORACLE_TTL=15s go run .
```

Env:
- `PORT` — listen port (default `8090`)
- `ORACLE_TTL` — cache freshness window (Go duration, default `30s`)
- `ORACLE_HTTP_TIMEOUT` — upstream request timeout (default `10s`)

## Endpoints

| Route | Description |
|---|---|
| `GET /health` | Liveness + the tracked symbol list. |
| `GET /prices` | All tracked prices with `updated_at`, `age_seconds`, `stale`. |
| `GET /prices/{symbol}` | One symbol, e.g. `/prices/BNB`. |
| `GET /prices/refresh` | Force an upstream refresh. |

```bash
curl -fsS localhost:8090/prices | jq
curl -fsS localhost:8090/prices/BTC
```

## Behavior

- Prices are cached; a request past the TTL transparently refreshes from upstream.
- If an upstream refresh fails but prices were previously cached, the stale
  snapshot is served with `"stale": true` rather than erroring (graceful
  degradation).
- Tracked universe: BNB, CAKE, ETH, BTC, USDT, XRP, ADA, DOGE, AVAX, LINK
  (mapped to their CoinGecko ids in `oracle/coingecko.go`).
