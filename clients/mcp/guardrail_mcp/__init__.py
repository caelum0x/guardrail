"""Stdlib-only MCP (Model Context Protocol) server for the Guardrail API.

Exposes the read-only Guardrail API as MCP tools over a JSON-RPC 2.0 stdio
transport so other agents (CMC Agent Hub / TWAK style) can consume Guardrail
over MCP. No external dependencies: the server reuses the dependency-free
Python SDK in ``clients/python/guardrail_client``.
"""

from __future__ import annotations

from .server import (
    SERVER_NAME,
    SERVER_VERSION,
    MCPServer,
    build_prompts,
    build_resources,
    build_tools,
    serve,
)

__all__ = [
    "SERVER_NAME",
    "SERVER_VERSION",
    "MCPServer",
    "build_tools",
    "build_resources",
    "build_prompts",
    "serve",
]
