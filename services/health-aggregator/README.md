# health-aggregator

A read-only Go ops service that polls the `/health` endpoints of the Guardrail
services concurrently and serves a single aggregated status. Only issues `GET`s;
no path into the trading loop.

## Run

```bash
cd services/health-aggregator
go run .                              # defaults: api :8080, price-oracle :8090, exporter :9100
HEALTH_TARGETS="api=http://127.0.0.1:8080/health,oracle=http://127.0.0.1:8090/health" go run .
HEALTH_ADDR=:9000 go run .
```

## Endpoints

| Route | Description |
|---|---|
| `GET /health` | Aggregate status (`200` if all up, `503` if any down) with per-target up/latency/error. |
| `GET /targets` | The configured target list. |

```bash
curl -fsS localhost:8095/health | jq
```

Each target is checked concurrently with a 5s client timeout; the aggregate is
`ok` only when every target returns 2xx, else `degraded`.
