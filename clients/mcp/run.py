#!/usr/bin/env python3
"""Entrypoint for the Guardrail MCP stdio server.

Run with::

    python3 clients/mcp/run.py

The server reads line-delimited JSON-RPC 2.0 requests from stdin and writes
responses to stdout. Configure the target API with the ``GUARDRAIL_BASE_URL``
environment variable (default ``http://localhost:8080``).
"""

from __future__ import annotations

import os
import sys

# Make the guardrail_mcp package importable when run as a standalone script.
_THIS_DIR = os.path.dirname(os.path.abspath(__file__))
if _THIS_DIR not in sys.path:
    sys.path.insert(0, _THIS_DIR)

from guardrail_mcp.server import serve  # noqa: E402


def main() -> None:
    serve()


if __name__ == "__main__":
    main()
