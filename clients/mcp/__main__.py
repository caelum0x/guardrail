"""Allow running the MCP server as ``python3 clients/mcp``.

Delegates to the same entrypoint as ``run.py``.
"""

from __future__ import annotations

import os
import sys

_THIS_DIR = os.path.dirname(os.path.abspath(__file__))
if _THIS_DIR not in sys.path:
    sys.path.insert(0, _THIS_DIR)

from guardrail_mcp.server import serve  # noqa: E402

if __name__ == "__main__":
    serve()
