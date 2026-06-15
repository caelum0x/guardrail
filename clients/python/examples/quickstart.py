#!/usr/bin/env python3
"""Quickstart for the stdlib-only Guardrail Python client.

Constructs a :class:`GuardrailClient` and prints ``health()`` and
``backtest()`` output. Network errors are caught so the script prints a notice
instead of crashing when the API is down.

Run from the repo root:

    python3 clients/python/examples/quickstart.py
"""

from __future__ import annotations

import json
import os
import sys

# Allow running directly from the source tree without installing.
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from guardrail_client import GuardrailApiError, GuardrailClient  # noqa: E402


def main() -> int:
    base_url = os.environ.get("GUARDRAIL_BASE_URL", "http://localhost:8080")
    client = GuardrailClient(base_url=base_url, timeout=10.0)

    print(f"Connecting to Guardrail API at {base_url}\n")

    try:
        health = client.health()
        print("health():")
        print(json.dumps(health, indent=2))
        print()

        backtest = client.backtest(steps=60, fear_greed=70, preset="balanced")
        print("backtest(steps=60, fear_greed=70, preset='balanced'):")
        print(json.dumps(backtest, indent=2))
    except GuardrailApiError as exc:
        print(
            "Notice: could not reach the Guardrail API "
            f"(is it running at {base_url}?)."
        )
        print(f"  Reason: {exc}")
        return 0

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
