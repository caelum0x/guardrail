# guardrail-langchain

LangChain tool integration for the **Guardrail Alpha** read-only API.

This package exposes Guardrail's research and status endpoints as agent tools.
It reuses the dependency-free Python SDK at
[`clients/python/guardrail_client`](../python/guardrail_client) and works
**with or without** `langchain` installed:

- With `langchain_core` installed, `build_tools()` returns
  `langchain_core.tools.StructuredTool` objects ready to hand to an agent.
- Without it, `build_tools()` returns plain `ToolSpec` dataclasses exposing
  `.name`, `.description` and `.func` -- so the module stays importable,
  usable and testable with **zero third-party dependencies**.

## Tools

`build_tools()` returns the following tools (each returns a JSON string):

| Tool | Wraps | Arguments |
|------|-------|-----------|
| `guardrail_health` | `/health` | none |
| `guardrail_backtest` | `/backtest` | `steps`, `fear_greed`, `preset` |
| `guardrail_walkforward` | `/walkforward` | `windows`, `steps`, `preset` |
| `guardrail_sweep` | `/sweep` | `steps`, `fear_greed` (list), `preset` |
| `guardrail_regime` | `/regime` | none |
| `guardrail_funding` | `/funding` | none |
| `guardrail_compete` | `/compete` | none |
| `guardrail_alerts` | `/alerts` | none |
| `guardrail_proof` | `/proof` | none |
| `guardrail_compile_policy` | `/policy/compile` | `mandate` (required) |
| `guardrail_indicators` | `/indicators` | `symbol`, `steps` |
| `guardrail_journal` | `/events` | none |
| `guardrail_scenarios` | `/scenarios` | none |
| `guardrail_skill_catalog` | embedded `skills/INDEX.json` | none |
| `guardrail_ensemble_routing` | `/regime` + embedded `skills/ensemble.json` | `regime` (optional) |

`guardrail_skill_catalog` lists the 5 strategy skills (id, name, thesis) from
the embedded `skills/INDEX.json` (anchored at the repo root — override with
`GUARDRAIL_REPO_ROOT`). It is fully offline and needs no API.

`guardrail_ensemble_routing` blends the embedded regime ensemble weights
(`skills/ensemble.json`, anchored at the repo root — override with
`GUARDRAIL_REPO_ROOT`). With a `regime` argument it resolves that regime
offline; without one it fetches the live `/regime` classification first.

> **Ensemble core vs. standalone sizing.** The ensemble core is the **four
> regime-complementary skills** (regime-routed alpha, funding-rate carry,
> mean-reversion-chop, trend-breakout-momentum) blended by
> `guardrail_ensemble_routing`. **`volatility-targeted-risk-parity`** is an
> *additional standalone sizing strategy* (how much to hold, not what) and is
> **not** part of the blended ensemble weights. The full 5-skill catalog is
> exposed by `guardrail_skill_catalog`.

## Installation

```bash
# Core only (stdlib + the local Python SDK; no third-party deps)
pip install .

# With LangChain support
pip install ".[langchain]"
```

Running directly from the source tree works too -- the package inserts the
sibling `clients/python` directory onto `sys.path` automatically.

## Usage without LangChain (fallback mode)

```python
import sys
sys.path.insert(0, "clients/langchain")

from guardrail_langchain import build_tools, langchain_available

tools = build_tools(base_url="http://localhost:8080")
print("langchain available:", langchain_available())  # False if not installed
print("tools:", [t.name for t in tools])

# Each ToolSpec is directly callable via .func
health_json = tools[0].func()   # guardrail_health takes no args
print(health_json)              # JSON string

# Tools that take arguments are called by keyword
bt = next(t for t in tools if t.name == "guardrail_backtest")
print(bt.func(steps=60, fear_greed=70, preset="balanced"))
```

## Usage with LangChain

When `langchain_core` is installed, the same call returns `StructuredTool`s:

```python
from guardrail_langchain import build_tools

tools = build_tools(base_url="http://localhost:8080")

# StructuredTool exposes .name / .description / .invoke(...)
health = next(t for t in tools if t.name == "guardrail_health")
print(health.invoke({}))
```

### Registering with a LangChain agent

```python
from langchain.agents import create_react_agent
from langchain_openai import ChatOpenAI

from guardrail_langchain import build_tools

tools = build_tools(base_url="http://localhost:8080")
llm = ChatOpenAI(model="gpt-4o-mini")

agent = create_react_agent(llm, tools)
result = agent.invoke(
    {"messages": [("user", "What is the current market regime and is the API healthy?")]}
)
print(result)
```

The agent can now call any Guardrail tool -- for example `guardrail_regime`
and `guardrail_health` -- and reason over the JSON each returns.

## Example script

```bash
python3 clients/langchain/examples/agent_demo.py
```

It builds the tools, prints their names, and calls `guardrail_health` if the
API is reachable (network errors are caught and reported).

## Notes

- The Guardrail API is **read-only**; these tools never mutate agent state.
- Tool errors (API down, non-2xx) are returned as a JSON error envelope rather
  than raised, so an agent can reason about the failure.
