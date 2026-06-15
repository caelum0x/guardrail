"""Guardrail LangChain integration.

Exposes the Guardrail Alpha read-only API as a set of agent tools. The package
reuses the stdlib-only Python SDK (``guardrail_client``) and works both with
and without ``langchain_core`` installed -- see :func:`build_tools`.
"""

from __future__ import annotations

from .tools import (
    DEFAULT_BASE_URL,
    ToolSpec,
    build_tools,
    langchain_available,
)

__all__ = [
    "build_tools",
    "ToolSpec",
    "langchain_available",
    "DEFAULT_BASE_URL",
]

__version__ = "0.3.0"
