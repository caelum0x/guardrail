#!/usr/bin/env python3
"""Guardrail edge gateway — a tiny stdlib reverse proxy in front of the read-only API.

Adds three things a public dashboard wants without touching the Rust API:
  * simple in-memory per-IP rate limiting,
  * permissive CORS headers (so the static cockpit / a hosted dashboard can call it),
  * short-TTL in-memory caching of read-only GET responses.

Read-only: only GET/HEAD/OPTIONS are proxied; anything else gets 405. Offline-safe
— importing and `--check` never require the upstream to be reachable. Standard
library only.

Usage:
    python3 services/gateway/gateway.py --check                 # validate config, exit 0
    python3 services/gateway/gateway.py --listen 8088 --upstream http://localhost:8080
"""

from __future__ import annotations

import argparse
import sys
import time
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

DEFAULT_LISTEN = 8088
DEFAULT_UPSTREAM = "http://localhost:8080"
DEFAULT_RATE = 60  # requests per window per client
DEFAULT_WINDOW = 60.0  # seconds
DEFAULT_CACHE_TTL = 5.0  # seconds


class _State:
    """Process-wide config + in-memory rate/cache state."""

    def __init__(self, upstream: str, rate: int, window: float, cache_ttl: float):
        self.upstream = upstream.rstrip("/")
        self.rate = rate
        self.window = window
        self.cache_ttl = cache_ttl
        self.hits: dict[str, list[float]] = {}
        self.cache: dict[str, tuple[float, int, bytes, str]] = {}

    def allow(self, client: str, now: float) -> bool:
        bucket = [t for t in self.hits.get(client, []) if now - t < self.window]
        bucket.append(now)
        self.hits[client] = bucket
        return len(bucket) <= self.rate


def _make_handler(state: _State):
    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def _cors(self) -> None:
            self.send_header("Access-Control-Allow-Origin", "*")
            self.send_header("Access-Control-Allow-Methods", "GET, HEAD, OPTIONS")
            self.send_header("Access-Control-Allow-Headers", "*")

        def do_OPTIONS(self) -> None:  # noqa: N802
            self.send_response(204)
            self._cors()
            self.send_header("Content-Length", "0")
            self.end_headers()

        def do_HEAD(self) -> None:  # noqa: N802
            self.do_GET(head=True)

        def do_GET(self, head: bool = False) -> None:  # noqa: N802
            now = time.monotonic()
            client = self.client_address[0]
            if not state.allow(client, now):
                self._send(429, b'{"error":"rate limited"}', "application/json", head)
                return

            cached = state.cache.get(self.path)
            if cached and now - cached[0] < state.cache_ttl:
                _, status, body, ctype = cached
                self._send(status, body, ctype, head, cache="HIT")
                return

            try:
                req = urllib.request.Request(state.upstream + self.path, method="GET")
                with urllib.request.urlopen(req, timeout=10) as resp:  # noqa: S310
                    body = resp.read()
                    status = resp.status
                    ctype = resp.headers.get("Content-Type", "application/json")
            except (urllib.error.URLError, OSError, TimeoutError) as exc:
                self._send(502, f'{{"error":"upstream: {exc}"}}'.encode(), "application/json", head)
                return

            if status == 200:
                state.cache[self.path] = (now, status, body, ctype)
            self._send(status, body, ctype, head, cache="MISS")

        def do_POST(self) -> None:  # noqa: N802
            self._send(405, b'{"error":"read-only gateway"}', "application/json", False)

        def _send(self, status, body, ctype, head, cache="BYPASS"):
            self.send_response(status)
            self._cors()
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("X-Cache", cache)
            self.end_headers()
            if not head:
                self.wfile.write(body)

        def log_message(self, *_args):  # silence default noisy logging
            pass

    return Handler


def check(state: _State, listen: int) -> int:
    """Validate config without binding a socket. Always exits 0 on valid config."""
    assert state.upstream.startswith(("http://", "https://")), "bad upstream"
    assert 1 <= listen <= 65535, "bad listen port"
    assert state.rate > 0 and state.window > 0, "bad rate/window"
    print(
        f"gateway config ok: listen :{listen} -> {state.upstream} "
        f"(rate {state.rate}/{state.window:g}s, cache {state.cache_ttl:g}s)"
    )
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Guardrail read-only edge gateway.")
    parser.add_argument("--listen", type=int, default=DEFAULT_LISTEN)
    parser.add_argument("--upstream", default=DEFAULT_UPSTREAM)
    parser.add_argument("--rate", type=int, default=DEFAULT_RATE)
    parser.add_argument("--window", type=float, default=DEFAULT_WINDOW)
    parser.add_argument("--cache-ttl", type=float, default=DEFAULT_CACHE_TTL)
    parser.add_argument("--check", action="store_true", help="validate config and exit 0")
    args = parser.parse_args(argv)

    state = _State(args.upstream, args.rate, args.window, args.cache_ttl)
    if args.check:
        return check(state, args.listen)

    server = ThreadingHTTPServer(("0.0.0.0", args.listen), _make_handler(state))
    print(f"gateway listening on :{args.listen} -> {state.upstream}", file=sys.stderr)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        server.shutdown()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
