# Guardrail Postman + REST Client Collection

Ready-to-run request collections for the read-only [Guardrail Alpha
API](../../docs/api/openapi.yaml). Every request is a side-effect-free `GET`,
one per route in the OpenAPI spec (58 routes), grouped into six folders:

| Folder                  | Covers                                                      |
| ----------------------- | ---------------------------------------------------------- |
| `readiness/competition` | `/readiness`, `/compete`, `/scorecard`, `/prizes`, ...     |
| `market data`           | `/assets`, `/quotes`, `/costs`, `/indicators`, `/regime`, ...|
| `strategy/backtest`     | `/backtest`, `/walkforward`, `/sweep`, `/optimize`, ...     |
| `risk/portfolio`        | `/portfolio`, `/risk`, `/drift`, `/exposure`, ...          |
| `identity/proof`        | `/proof`, `/agent-card`, `/bnb-sdk`, `/skill`, ...         |
| `ops`                   | `/health`, `/events`, `/cockpit`, `/metrics`, ...          |

Two equivalent formats are provided:

- `guardrail.postman_collection.json` — Postman v2.1 collection.
- `guardrail.http` — VS Code REST Client / JetBrains HTTP Client file.

Both default to `http://localhost:8080`. Start the API first (from repo root):

```bash
cargo run -p guardrail-api
```

## Importing into Postman

1. Open Postman -> **Import** (top-left).
2. Drag in `clients/postman/guardrail.postman_collection.json`, or use
   **File -> Upload Files** and select it.
3. The collection **Guardrail Alpha API** appears with the six folders above.
4. The `baseUrl` collection variable defaults to `http://localhost:8080`. To
   point at another host, open the collection -> **Variables** tab and edit the
   `baseUrl` current value (e.g. `http://127.0.0.1:9000`).
5. Open any request and click **Send**.

Requests with query parameters (`/backtest`, `/walkforward`, `/sweep`,
`/optimize`, `/rebalance`, `/costs`, `/quotes`, `/indicators`, `/watchlist`,
`/policy/compile`) ship their query keys in the request's **Params** tab.
Optional parameters are pre-filled with the spec defaults but **disabled** so
the request runs with server defaults; tick the checkbox to send them. The one
required parameter (`mandate` on `/policy/compile`) is enabled by default.

## Using the `.http` file (VS Code / JetBrains)

### VS Code REST Client

1. Install the **REST Client** extension (`humao.rest-client`).
2. Open `clients/postman/guardrail.http`.
3. Edit the `@baseUrl` line at the top if your server is elsewhere.
4. Click the **Send Request** link that appears above any `GET` line.

### JetBrains IDEs (IntelliJ, GoLand, PyCharm, ...)

1. Open `clients/postman/guardrail.http` (built-in HTTP Client, no plugin).
2. Adjust `@baseUrl` if needed.
3. Click the green **Run** gutter icon next to any request.

Each request is separated by a `###` line (the standard REST Client / JetBrains
request delimiter). Optional query parameters are documented in a comment above
each request so you can append them by hand, e.g.:

```http
### /backtest
# optional query params: steps (default 60), fear_greed (default 60), preset (default balanced)
GET {{baseUrl}}/backtest?steps=120&fear_greed=75&preset=balanced
Accept: application/json
```

## Regenerating

Both files are generated from `docs/api/openapi.yaml`. If the route list
changes, re-run the generator that produced them (it maps every path in the
spec into the six folders and writes both formats). Keeping them derived from
the spec guarantees the request set stays in lockstep with the server.
