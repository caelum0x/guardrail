# notification-relay

A concurrent webhook fan-out relay. An incoming JSON alert posted to
`/notify` is delivered concurrently to every configured webhook target,
with per-target timeout and retry, and a per-target delivery report is
returned. Standard library only.

## Build & verify

```sh
go vet ./...
go build ./...
```

## Run

```sh
NOTIFY_WEBHOOKS="https://hooks.example.com/a,https://hooks.example.com/b" \
  go run .
```

## Configuration (environment variables)

| Variable          | Default   | Description                                          |
|-------------------|-----------|------------------------------------------------------|
| `NOTIFY_WEBHOOKS` | (required)| Comma-separated webhook URLs (`http://`/`https://`). |
| `NOTIFY_ADDR`     | `:8085`   | Listen address.                                      |
| `NOTIFY_TIMEOUT`  | `5s`      | Per-target attempt timeout (Go duration, max `60s`). |
| `NOTIFY_RETRIES`  | `2`       | Extra attempts after the first failure (max `10`).   |
| `NOTIFY_BACKOFF`  | `250ms`   | Base backoff between attempts (grows linearly).      |

## Endpoints

### `POST /notify`

Body: any valid JSON alert (max 1 MiB). The exact bytes are forwarded to
each target as `application/json`.

Fan-out is concurrent (one goroutine per target, joined with
`sync.WaitGroup`). Each target gets up to `NOTIFY_RETRIES + 1` attempts
with linear backoff; only 2xx responses count as delivered.

Response status:

- `200 OK` — all targets delivered.
- `207 Multi-Status` — some delivered, some failed.
- `502 Bad Gateway` — all targets failed.

Response body (delivery report):

```json
{
  "total": 2,
  "delivered": 1,
  "failed": 1,
  "duration_ms": 312,
  "results": [
    {
      "target": "https://hooks.example.com/a",
      "success": true,
      "status_code": 200,
      "attempts": 1,
      "duration_ms": 110,
      "response": "ok"
    },
    {
      "target": "https://hooks.example.com/b",
      "success": false,
      "status_code": 0,
      "attempts": 3,
      "duration_ms": 5200,
      "error": "deliver: context deadline exceeded"
    }
  ]
}
```

### `GET /health`

```json
{
  "status": "ok",
  "targets": 2,
  "urls": ["https://hooks.example.com/a", "https://hooks.example.com/b"],
  "time": "2026-06-16T00:00:00Z"
}
```

## Example

```sh
curl -s -X POST localhost:8085/notify \
  -H 'Content-Type: application/json' \
  -d '{"level":"critical","message":"risk limit breached"}' | jq
```

## Layout

```
notification-relay/
├── go.mod              module guardrail/notification-relay
├── main.go             env config, HTTP server, graceful shutdown
├── relay/
│   ├── relay.go        Config, Relay, concurrent Fanout + retry/backoff
│   └── server.go       /notify + /health HTTP handlers
└── README.md
```
