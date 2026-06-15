#!/usr/bin/env python3
"""End-to-end quickstart for the stdlib-only Guardrail Python SDK.

This runs a guided sequence against the read-only Guardrail Alpha API and
prints a concise summary of each call:

    1. health()          -- API + database status
    2. compile_policy()  -- compile a natural-language mandate into a policy hash
    3. backtest()        -- strategy vs benchmark over 60 steps
    4. walkforward()     -- rolling out-of-sample windows
    5. regime()          -- current market regime
    6. compete()         -- competition status

The script reuses the published SDK (``clients/python/guardrail_client``) by
inserting that directory on ``sys.path`` -- no install required. Every call is
wrapped so that an unreachable / down API prints a friendly notice and exits 0
rather than emitting a stack trace.

Run from the repo root (start the API first: ``cargo run -p guardrail-api``)::

    python3 clients/examples/python_quickstart.py

Configure the target with the ``GUARDRAIL_BASE_URL`` environment variable
(default ``http://localhost:8080``).
"""

from __future__ import annotations

import os
import sys
from typing import Any, Dict

# Reuse the existing SDK from ../python without installing it.
_SDK_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "python")
sys.path.insert(0, _SDK_DIR)

from guardrail_client import GuardrailApiError, GuardrailClient  # noqa: E402


def _fmt(value: Any) -> str:
    """Render a scalar field for a one-line summary."""
    if value is None:
        return "n/a"
    return str(value)


def _summarize_health(data: Dict[str, Any]) -> str:
    ok = data.get("ok")
    events = data.get("events_visible")
    return f"ok={_fmt(ok)} events_visible={_fmt(events)}"


def _summarize_policy(data: Dict[str, Any]) -> str:
    if data.get("error"):
        return f"error={data['error']}"
    return f"hash={_fmt(data.get('hash'))}"


def _summarize_backtest(data: Dict[str, Any]) -> str:
    metrics = data.get("metrics") or {}
    return (
        f"steps={_fmt(data.get('steps'))} "
        f"final_nav_usd={_fmt(data.get('final_nav_usd'))} "
        f"total_return_pct={_fmt(metrics.get('total_return_pct'))} "
        f"max_drawdown_pct={_fmt(metrics.get('max_drawdown_pct'))} "
        f"excess_return_pct={_fmt(data.get('excess_return_pct'))}"
    )


def _summarize_walkforward(data: Dict[str, Any]) -> str:
    windows = data.get("windows") or []
    agg = data.get("aggregate") or {}
    return (
        f"windows={len(windows)} "
        f"mean_excess_pct={_fmt(agg.get('mean_excess_pct'))} "
        f"positive_windows={_fmt(agg.get('positive_windows'))}"
    )


def _summarize_regime(data: Dict[str, Any]) -> str:
    # The regime payload shape is open-ended; surface the most useful keys
    # when present and otherwise list the top-level keys.
    for key in ("regime", "label", "state", "name"):
        if key in data:
            return f"{key}={_fmt(data[key])}"
    keys = ", ".join(sorted(data.keys())) or "(empty)"
    return f"keys: {keys}"


def _summarize_compete(data: Dict[str, Any]) -> str:
    for key in ("status", "competition", "rank", "name"):
        if key in data:
            return f"{key}={_fmt(data[key])}"
    keys = ", ".join(sorted(data.keys())) or "(empty)"
    return f"keys: {keys}"


def main() -> int:
    base_url = os.environ.get("GUARDRAIL_BASE_URL", "http://localhost:8080")
    client = GuardrailClient(base_url=base_url, timeout=10.0)

    print(f"Guardrail Python SDK quickstart -> {base_url}\n")

    try:
        print("[1/6] health()")
        print("      " + _summarize_health(client.health()))

        mandate = "Trade CAKE max drawdown 20% kill switch 25%"
        print(f"\n[2/6] compile_policy({mandate!r})")
        print("      " + _summarize_policy(client.compile_policy(mandate)))

        print("\n[3/6] backtest(steps=60, fear_greed=70, preset='balanced')")
        print(
            "      "
            + _summarize_backtest(
                client.backtest(steps=60, fear_greed=70, preset="balanced")
            )
        )

        print("\n[4/6] walkforward()")
        print("      " + _summarize_walkforward(client.walkforward()))

        print("\n[5/6] regime()")
        print("      " + _summarize_regime(client.regime()))

        print("\n[6/6] compete()")
        print("      " + _summarize_compete(client.compete()))

    except GuardrailApiError as exc:
        print("\nNotice: could not complete the sequence against the Guardrail API.")
        print(f"  Is it running at {base_url}? Start it with: cargo run -p guardrail-api")
        print(f"  Reason: {exc}")
        return 0

    print("\nDone. All calls completed successfully.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
