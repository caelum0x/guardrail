# Guardrail Edge Gateway (read-only)

A tiny standard-library reverse proxy that fronts the read-only Guardrail API for
public/dashboard access, adding in-memory **rate limiting**, permissive **CORS**,
and short-TTL **response caching** — without touching the Rust API. Only
`GET`/`HEAD`/`OPTIONS` are proxied; writes get `405`.

## Run
```bash
# validate config, no socket bind (offline-safe)
python3 services/gateway/gateway.py --check

# serve on :8088 in front of the API
python3 services/gateway/gateway.py --listen 8088 --upstream http://localhost:8080
```

## Flags
| Flag | Default | Meaning |
|---|---|---|
| `--listen` | `8088` | port to bind |
| `--upstream` | `http://localhost:8080` | the guardrail-api base |
| `--rate` / `--window` | `60` / `60s` | per-IP request budget |
| `--cache-ttl` | `5s` | GET response cache TTL |
| `--check` | — | validate config + exit 0 (no bind) |

Responses carry an `X-Cache: HIT|MISS|BYPASS` header. Standard library only.
