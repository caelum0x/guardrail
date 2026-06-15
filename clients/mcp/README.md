# Guardrail MCP Server

A pure-stdlib [Model Context Protocol](https://modelcontextprotocol.io) (MCP)
server that exposes the **read-only Guardrail API** as MCP **tools**,
**resources** and **prompts**. It lets other agents (CMC Agent Hub / TWAK style)
consume Guardrail over MCP without any network or SDK glue of their own.

- **Transport:** JSON-RPC 2.0 over stdio (line-delimited JSON).
- **Dependencies:** none. Standard library only. It reuses the dependency-free
  Python SDK in [`clients/python/guardrail_client`](../python/guardrail_client),
  imported via a `sys.path` insert.
- **Read-only:** every tool and resource maps to a read-only Guardrail API
  route. The server never mutates agent state.
- **Capabilities:** `initialize` advertises `tools`, `resources` and `prompts`.
- **Hub manifest:** see [`manifest.json`](./manifest.json) for the machine-
  readable catalog and [`../../docs/MCP_HUB.md`](../../docs/MCP_HUB.md) for hub
  registration instructions.

## Tools

| Tool | Inputs | Description |
|------|--------|-------------|
| `guardrail_health` | — | API and database health status. |
| `guardrail_backtest` | `steps?`, `fear_greed?`, `preset?` | Strategy-vs-benchmark backtest. |
| `guardrail_walkforward` | `windows?`, `steps?`, `preset?` | Rolling walk-forward windows. |
| `guardrail_sweep` | `steps?`, `fear_greed?` (int[]), `preset?` | Sentiment comparison sweep. |
| `guardrail_regime` | — | Current market regime classification. |
| `guardrail_funding` | — | Funding rates across markets. |
| `guardrail_compete` | — | Competition status and standings. |
| `guardrail_alerts` | — | Currently evaluated alerts. |
| `guardrail_proof` | — | Agent identity + report proof. |
| `guardrail_compile_policy` | `mandate` (required) | Compile a natural-language mandate into a validated policy + hash. |
| `guardrail_indicators` | `symbol?`, `steps?` | Deterministic synthetic indicators for a symbol. |
| `guardrail_skill` | — | Agent skill descriptor / capability catalog. |
| `guardrail_prizes` | — | Hackathon prize-track catalog and mapping. |
| `guardrail_readiness` | — | Operational readiness probe for the agent. |
| `guardrail_journal` | — | Decision journal: the agent's recent event log (`/events`). |
| `guardrail_scenarios` | — | Stress-scenario catalog and each scenario's expected risk response. |
| `guardrail_skill_catalog` | — | List the 5 strategy skills (id, name, thesis) from `skills/INDEX.json`. |
| `guardrail_ensemble_routing` | `regime?` | Resolve regime-routed ensemble weights — blends the embedded per-skill weights (`skills/ensemble.json`) for the live `/regime` classification, or for a `regime` you pass in. |

> **Ensemble core vs. standalone sizing.** The ensemble core is the **four
> regime-complementary skills** (regime-routed alpha, funding-rate carry,
> mean-reversion-chop, trend-breakout-momentum) blended by
> `guardrail_ensemble_routing`. **`volatility-targeted-risk-parity`** is an
> *additional standalone sizing strategy* on a different axis (how much, not
> what) — it is **not** part of the blended ensemble weights. The full 5-skill
> catalog is exposed by `guardrail_skill_catalog` and the `guardrail://skills`
> resource.

All inputs are optional unless marked **required**. Each tool advertises a JSON
Schema `inputSchema` via `tools/list`.

## Resources

Resources expose read-only Guardrail artifacts under the `guardrail://` URI
scheme. `resources/list` advertises them; `resources/read` fetches the
underlying API route and returns its content. When the API is unreachable, a
read returns a JSON error payload as the resource content (it never crashes the
transport).

| URI | Route | Description |
|-----|-------|-------------|
| `guardrail://compete` | `/compete` | Live competition standings and status. |
| `guardrail://regime` | `/regime` | Current market regime classification. |
| `guardrail://skill` | `/skill` | Agent skill descriptor / capability catalog. |
| `guardrail://prizes` | `/prizes` | Hackathon prize-track catalog and mapping. |
| `guardrail://readiness` | `/readiness` | Operational readiness probe. |
| `guardrail://ensemble` | `skills/ensemble.json` (file) | Embedded meta-allocator weights blending the four regime-complementary ensemble-core skills by regime. |
| `guardrail://scenarios` | `configs/scenarios/index.json` (file) | Committed stress-scenario catalog with expected risk responses. |
| `guardrail://skills` | `skills/INDEX.json` (file) | Full Track-2 strategy skill catalog (5 skills): the four ensemble-core skills plus the standalone `volatility-targeted-risk-parity` sizing strategy. |

The last three resources have **no API route** — they read committed artifacts
off disk (anchored at the repo root; override with `GUARDRAIL_REPO_ROOT`). They
therefore work fully offline, without the API running.

## Prompts

Prompts are reusable message templates. `prompts/list` advertises them with
typed arguments; `prompts/get` renders a template into MCP messages. Renderers
are pure (no network), so prompts work without a live API.

| Prompt | Arguments | Description |
|--------|-----------|-------------|
| `analyze-regime` | `focus?`, `horizon?` | Analyze the current market regime and risk implications. |
| `explain-decision` | `decision?`, `audience?` | Explain an agent decision with grounded evidence. |
| `draft-mandate` | `objective?`, `risk_tolerance?`, `constraints?` | Draft a compilable Guardrail mandate. |
| `explain-ensemble-routing` | `regime?`, `audience?` | Explain how the regime-routed ensemble blends the strategy skills. |
| `summarize-journal` | `window?`, `focus?` | Summarize the decision journal (`/events`) into a timeline. |

## Running

```bash
python3 clients/mcp/run.py
# or, as a module:
python3 clients/mcp
```

Configure the target API base URL with an environment variable (default
`http://localhost:8080`):

```bash
export GUARDRAIL_BASE_URL=https://your-guardrail-host
python3 clients/mcp/run.py
```

### Quick handshake check

```bash
printf '%s\n%s\n%s\n%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  '{"jsonrpc":"2.0","id":3,"method":"resources/list","params":{}}' \
  '{"jsonrpc":"2.0","id":4,"method":"prompts/list","params":{}}' \
  | python3 clients/mcp/run.py
```

This prints four JSON-RPC results: `initialize` advertises the `tools`,
`resources` and `prompts` capabilities; the remaining three list the available
tools, resources and prompts. The Guardrail API does **not** need to be running
for any list call or for `initialize` / `prompts/get` — only `tools/call` and
`resources/read` reach the backend.

## Registering with an MCP client

The server speaks stdio, so register it as a stdio MCP server. A sample config
entry lives in [`mcp.json`](./mcp.json):

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

Adjust the `args` path to an absolute path if your MCP client does not run from
the repository root.

## Protocol details

- **`initialize`** returns `serverInfo` (`{"name": "guardrail-mcp", "version": ...}`)
  and `capabilities` (`{"tools": {...}, "resources": {...}, "prompts": {...}}`).
- **`tools/list`** advertises the tools above, each with a JSON-Schema
  `inputSchema`.
- **`tools/call`** dispatches to the matching SDK method and returns the JSON
  result as `{"content": [{"type": "text", "text": "<json>"}]}`.
- **`resources/list`** advertises the `guardrail://` resources, each with a
  `uri`, `name`, `description` and `mimeType`.
- **`resources/read`** fetches the underlying route and returns
  `{"contents": [{"uri": ..., "mimeType": ..., "text": "<json>"}]}`. If the API
  is down, the content is a JSON error payload (the transport never crashes).
- **`prompts/list`** advertises the prompt templates and their typed arguments.
- **`prompts/get`** renders a template into `{"description": ..., "messages": [...]}`
  using the supplied (optional) arguments.
- On API or argument errors the server returns an MCP tool error
  (`isError: true`) with a descriptive message — it never crashes the transport.
- Notifications (requests without an `id`, e.g. `notifications/initialized`)
  produce no response, per JSON-RPC 2.0.
