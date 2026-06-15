# Guardrail MCP Hub Guide

This document describes the **Guardrail MCP server** — what it exposes and how
to register it with an MCP host such as the **CoinMarketCap (CMC) Agent Hub**, a
TWAK-style host, Claude Desktop, or any client that speaks the
[Model Context Protocol](https://modelcontextprotocol.io).

The server is pure standard library (no third-party dependencies) and wraps the
**read-only** Guardrail API. It speaks JSON-RPC 2.0 over stdio.

- Server code: [`clients/mcp/`](../clients/mcp/)
- Machine-readable manifest: [`clients/mcp/manifest.json`](../clients/mcp/manifest.json)
- Client README: [`clients/mcp/README.md`](../clients/mcp/README.md)

## What it exposes

The server advertises three MCP capabilities in its `initialize` result:
`tools`, `resources` and `prompts`.

### Tools (14)

Each tool maps to a single read-only Guardrail API route and returns the JSON
response as a text content block. Argument and API errors are returned as MCP
tool errors (`isError: true`) — the transport never crashes.

`guardrail_health`, `guardrail_backtest`, `guardrail_walkforward`,
`guardrail_sweep`, `guardrail_regime`, `guardrail_funding`, `guardrail_compete`,
`guardrail_alerts`, `guardrail_proof`, `guardrail_compile_policy`,
`guardrail_indicators`, `guardrail_skill`, `guardrail_prizes`,
`guardrail_readiness`.

See the [client README](../clients/mcp/README.md#tools) for the full input
schemas.

### Resources (5)

Read-only Guardrail artifacts under the `guardrail://` URI scheme. `resources/read`
fetches the underlying route on demand. When the API is unreachable the read
returns a JSON error payload as the resource content (still well-formed).

| URI | Backing route | Description |
|-----|---------------|-------------|
| `guardrail://compete` | `/compete` | Live competition standings and status. |
| `guardrail://regime` | `/regime` | Current market regime classification. |
| `guardrail://skill` | `/skill` | Agent skill descriptor / capability catalog. |
| `guardrail://prizes` | `/prizes` | Hackathon prize-track catalog and mapping. |
| `guardrail://readiness` | `/readiness` | Operational readiness probe. |

### Prompts (3)

Reusable, parameterized message templates. Rendering is pure (no network), so
`prompts/get` works without a live API.

| Prompt | Arguments | Purpose |
|--------|-----------|---------|
| `analyze-regime` | `focus?`, `horizon?` | Analyze the current market regime and risk implications. |
| `explain-decision` | `decision?`, `audience?` | Explain a Guardrail agent decision with grounded evidence. |
| `draft-mandate` | `objective?`, `risk_tolerance?`, `constraints?` | Draft a compilable natural-language mandate. |

## Environment configuration

The server proxies a Guardrail API instance selected by one environment
variable:

| Variable | Default | Description |
|----------|---------|-------------|
| `GUARDRAIL_BASE_URL` | `http://localhost:8080` | Base URL of the read-only Guardrail API. |

No secrets are required: the server only calls read-only routes and never
mutates agent state.

## Registering with an MCP host

The server runs as a stdio MCP server. Point your host's command at
`clients/mcp/run.py` (use an absolute path if the host does not run from the
repository root).

### Generic stdio MCP config

A ready-to-copy entry lives in [`clients/mcp/mcp.json`](../clients/mcp/mcp.json):

```json
{
  "mcpServers": {
    "guardrail": {
      "command": "python3",
      "args": ["clients/mcp/run.py"],
      "env": {
        "GUARDRAIL_BASE_URL": "http://localhost:8080"
      }
    }
  }
}
```

### CMC Agent Hub / TWAK-style host

Hubs that ingest a server manifest can read
[`clients/mcp/manifest.json`](../clients/mcp/manifest.json) directly. It declares
the server `name`, `version`, `description`, the `stdio` transport, the
`runtime` command/args, the `GUARDRAIL_BASE_URL` env contract, the advertised
capability flags, and the full tool / resource / prompt catalog.

To register:

1. Ensure `python3` (3.8+) is available on the host. No `pip install` step is
   needed — the server is stdlib-only.
2. Set `GUARDRAIL_BASE_URL` to your deployed Guardrail API endpoint.
3. Provide the runtime command from the manifest
   (`python3 clients/mcp/run.py`), using an absolute path to `run.py`.
4. The hub discovers tools/resources/prompts at runtime via `tools/list`,
   `resources/list` and `prompts/list`.

### Claude Desktop

Add the same `mcpServers` block (above) to your
`claude_desktop_config.json`, using an absolute path for `run.py`.

## Smoke test (no live API required)

Pipe JSON-RPC requests over stdio and confirm the four discovery responses.
`initialize`, `tools/list`, `resources/list` and `prompts/list` all work
without the backend; only `tools/call` and `resources/read` reach it.

```bash
printf '%s\n%s\n%s\n%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  '{"jsonrpc":"2.0","id":3,"method":"resources/list","params":{}}' \
  '{"jsonrpc":"2.0","id":4,"method":"prompts/list","params":{}}' \
  | python3 clients/mcp/run.py
```

Expected:

- response `1` advertises `capabilities.tools`, `capabilities.resources` and
  `capabilities.prompts`;
- response `2` lists 14 tools;
- response `3` lists 5 resources;
- response `4` lists 3 prompts.

Validate the manifest parses:

```bash
python3 -c "import json; json.load(open('clients/mcp/manifest.json')); print('manifest OK')"
```

## Design notes

- **Stdlib only.** The server reuses the dependency-free Python SDK in
  [`clients/python/guardrail_client`](../clients/python/guardrail_client),
  imported via a `sys.path` insert. No `requests`/`httpx`/MCP SDK.
- **Read-only & non-mutating.** Every tool and resource maps to a read-only
  route.
- **Resilient transport.** API outages surface as tool errors
  (`isError: true`) or as JSON error payloads in resource contents; the stdio
  loop keeps running. Notifications (no `id`) produce no response, per
  JSON-RPC 2.0.
