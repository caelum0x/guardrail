# Real-Time Telemetry (SSE)

The dashboard can stream the agent's decisions live instead of polling.

## `GET /stream` (guardrail-api)

A [Server-Sent Events](https://developer.mozilla.org/docs/Web/API/Server-sent_events)
endpoint (`text/event-stream`). It **tails the append-only SQLite event log
cross-process**: the agent writes events to the DB, and the API streams them to
connected browsers. Implementation: `apps/guardrail-api/src/stream.rs`.

- On connect it replays the most recent events (oldest-first), then on a ~1s
  interval emits any events newer than the last one sent.
- Each frame is `event: agent_event` with a JSON `StoredEvent` `data:` payload
  (`{id, run_id, timestamp, event_type, payload_json}`).
- A periodic keep-alive comment holds idle connections open.
- Read-only, non-blocking, and it ends cleanly when the client disconnects.

Because it tails the DB (rather than an in-process channel), it works even though
the agent and API are separate processes.

## Dashboard `/live`

`dashboard/src/app/live/page.tsx` (a client component) subscribes to `/stream`
via the native `EventSource` API (`dashboard/src/lib/stream.ts`) and renders a
live, auto-updating feed: current regime, rolling event list, live NAV and trade
count. It degrades to "waiting for /stream…" when the stream is unavailable and
never crashes. No extra npm dependencies.

The zero-dependency `clients/web-lite` cockpit polls the same read-only routes for
environments where SSE isn't desired.
