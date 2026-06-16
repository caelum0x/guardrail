#!/usr/bin/env python3
"""Demonstrate the Guardrail quant endpoints via the Python SDK.

Run a Guardrail API locally first (`cargo run -p guardrail-api`), then:

    python3 clients/python/examples/quant_tools.py

Every call is read-only. Set GUARDRAIL_API to point at a non-default host.
"""

from __future__ import annotations

import os
import sys

# Allow running from the repo root without installing the package.
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from guardrail_client import GuardrailClient  # noqa: E402


def main() -> int:
    client = GuardrailClient(base_url=os.environ.get("GUARDRAIL_API", "http://127.0.0.1:8080"))

    try:
        rsi = client.ta("rsi", [44, 44.3, 44.1, 43.6, 44.3, 44.8, 45.1, 45.4, 45.8, 46.0], period=5)
        print("RSI:", rsi.get("result", rsi))

        cost = client.fees(notional_usd=25000, quantity=12, side="buy")
        print("swap cost:", cost.get("breakdown", cost))

        size = client.sizer("kelly", win_prob=0.6, odds=1.5)
        print("kelly size:", size.get("output", size))

        pnl = client.pnl(fills="CAKE,buy,10,2;CAKE,sell,4,3", marks="CAKE:3")
        print("pnl total:", pnl.get("report", {}).get("total", pnl))
    except Exception as err:  # noqa: BLE001 - example: surface any connection error plainly
        print(f"error (is the API running?): {err}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
