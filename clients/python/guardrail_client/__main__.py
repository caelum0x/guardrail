"""Module entrypoint so the CLI runs as ``python -m guardrail_client``."""

from __future__ import annotations

import sys

from .cli import main

if __name__ == "__main__":
    sys.exit(main())
