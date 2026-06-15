#!/usr/bin/env python3
"""Runnable demo for the Guardrail LangChain tools.

Builds the tool set, prints the tool names, and -- if the Guardrail API is
reachable -- invokes one tool (``guardrail_health``) and prints its result.
Network errors are caught so the script prints a notice instead of crashing
when the API is down. Works with or without ``langchain_core`` installed.

Run from the repo root:

    python3 clients/langchain/examples/agent_demo.py
"""

from __future__ import annotations

import os
import sys

# Allow running directly from the source tree without installing.
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from guardrail_langchain import build_tools, langchain_available  # noqa: E402


def _invoke(tool: object) -> str:
    """Invoke a tool regardless of whether it is a StructuredTool or ToolSpec.

    LangChain tools expose ``.invoke({...})``; the fallback ToolSpec exposes a
    plain callable ``.func``.
    """
    invoke = getattr(tool, "invoke", None)
    if callable(invoke):
        return invoke({})
    return tool.func()  # type: ignore[attr-defined]


def main() -> int:
    base_url = os.environ.get("GUARDRAIL_BASE_URL", "http://localhost:8080")
    tools = build_tools(base_url=base_url)

    backend = "langchain_core" if langchain_available() else "fallback (ToolSpec)"
    print(f"Backend: {backend}")
    print(f"Built {len(tools)} Guardrail tools:")
    for tool in tools:
        print(f"  - {tool.name}: {tool.description.splitlines()[0]}")
    print()

    # Find the health tool and try calling it.
    health_tool = next((t for t in tools if t.name == "guardrail_health"), None)
    if health_tool is None:
        print("No guardrail_health tool found; nothing to call.")
        return 0

    print(f"Calling guardrail_health against {base_url} ...")
    try:
        result = _invoke(health_tool)
        print(result)
    except Exception as exc:  # noqa: BLE001 - demo guards all network errors
        print(
            "Notice: could not reach the Guardrail API "
            f"(is it running at {base_url}?)."
        )
        print(f"  Reason: {type(exc).__name__}: {exc}")

    return 0


def register_with_agent_example() -> None:
    """Illustrative-only: how you'd register these tools with a LangChain agent.

    Not executed by ``main`` -- it requires ``langchain`` and a configured LLM.

        from langchain.agents import create_react_agent
        from langchain_openai import ChatOpenAI

        tools = build_tools(base_url="http://localhost:8080")
        llm = ChatOpenAI(model="gpt-4o-mini")
        agent = create_react_agent(llm, tools)
    """


if __name__ == "__main__":
    raise SystemExit(main())
