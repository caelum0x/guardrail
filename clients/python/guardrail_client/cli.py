"""guardrail — a stdlib-only operator CLI for the Guardrail Alpha API.

This mirrors the TypeScript ``guardrail`` CLI and the Go ``guardrailctl``: it
uses only the standard library plus this package's :class:`GuardrailClient`, and
is **offline-safe** by design. Every subcommand except ``smoke`` prints a notice
and exits ``0`` when the API is unreachable, so it is harmless in CI or against a
stopped backend. ``smoke`` is the lone exception: a pre-ship gate that exits
non-zero when any quant endpoint fails to respond.

Run it as a module::

    python -m guardrail_client status
    python -m guardrail_client smoke --base http://127.0.0.1:8091
    python -m guardrail_client regime --json

Subcommands:
    status   /health + /readiness + /regime summary lines
    regime   current market regime and its inputs
    smoke    exercise every quant endpoint; PASS/WARN/FAIL table (gate)
    help     show usage
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from typing import Any, Callable, Dict, List, Optional, Tuple

from . import DEFAULT_BASE_URL, GuardrailApiError, GuardrailClient

# Only two non-gate exit codes. Operational failures (API down, decode errors)
# deliberately still exit 0 for every command except ``smoke`` so the tool is
# safe to run offline; a usage mistake exits 2 and a smoke failure exits 1.
EXIT_OK = 0
EXIT_SMOKE_FAIL = 1
EXIT_USAGE = 2

# A short per-call timeout so an unreachable host fails fast rather than hanging.
REQUEST_TIMEOUT_S = 5.0


def _resolve_base(explicit: Optional[str]) -> str:
    """Resolve the API base URL: --base > $GUARDRAIL_BASE_URL > default."""
    if explicit:
        return explicit
    env = os.environ.get("GUARDRAIL_BASE_URL", "").strip()
    return env if env else DEFAULT_BASE_URL


def _print_json(value: Any) -> None:
    print(json.dumps(value, indent=2, sort_keys=True))


def _unavailable(label: str, error: Exception) -> str:
    return f"{label}: unavailable: {error}"


# --- Read commands (offline-safe) --------------------------------------------
def cmd_status(client: GuardrailClient, as_json: bool) -> int:
    health = _try(client.health)
    readiness = _try(client.readiness)
    regime = _try(client.regime)

    if as_json:
        _print_json(
            {
                "health": health or {"status": "offline"},
                "readiness": readiness or {"status": "offline"},
                "regime": regime or {"status": "offline"},
            }
        )
        return EXIT_OK

    if regime is not None:
        print(
            f"regime: {regime.get('regime', '?')}  "
            f"(exposure x{regime.get('exposure_multiplier', '?')})"
        )
    else:
        print("regime: offline")

    if readiness is None:
        print("readiness: offline")
    else:
        print(
            f"readiness: {readiness.get('status', '?')}  "
            f"({readiness.get('blocking', 0)} blocking)"
        )

    if health is None:
        print("health: offline")
    else:
        ok = health.get("ok")
        print(f"health: {'ok' if ok else 'degraded'}  "
              f"(events_visible={health.get('events_visible', 0)})")
    return EXIT_OK


def cmd_regime(client: GuardrailClient, as_json: bool) -> int:
    try:
        regime = client.regime()
    except GuardrailApiError as error:
        print(_unavailable("regime", error))
        return EXIT_OK

    if as_json:
        _print_json(regime)
        return EXIT_OK

    print(f"regime: {regime.get('regime', '?')}")
    print(f"  exposure multiplier: {regime.get('exposure_multiplier', '?')}")
    inputs = regime.get("inputs", {})
    if isinstance(inputs, dict):
        print(
            "  inputs: "
            f"f/g={inputs.get('fear_greed', '?')} "
            f"breadth={inputs.get('breadth_pct', '?')}% "
            f"btc_dom={inputs.get('btc_dominance_pct', '?')}% "
            f"median_24h={inputs.get('median_24h_return', '?')}"
        )
    return EXIT_OK


# --- Smoke gate (NOT offline-safe) -------------------------------------------
# Mirrors scripts/smoke_quant.sh, the TS CLI `smoke`, and guardrailctl `smoke`:
# the same nine read-only quant endpoints with inputs that produce a real
# (non-error) response.
SmokeCall = Callable[[GuardrailClient], Dict[str, Any]]
SMOKE_CHECKS: List[Tuple[str, SmokeCall]] = [
    ("ta", lambda c: c.ta("rsi", [44, 44.3, 44.1, 43.6, 44.3, 44.8], period=5)),
    ("fees", lambda c: c.fees(notional_usd=25000, quantity=12, side="buy")),
    ("sizer", lambda c: c.sizer("kelly", win_prob=0.6, odds=1.5)),
    ("orderbook", lambda c: c.orderbook("s,limit,101,5;b,market,,6")),
    ("pnl", lambda c: c.pnl("CAKE,buy,10,2;CAKE,sell,4,3", "CAKE:3")),
    ("correlation", lambda c: c.correlation("BTC:0.01,-0.02,0.03;ETH:0.012,-0.018,0.025")),
    ("equity/indicators", lambda c: c.equity_indicators("rsi", 14)),
    ("portfolio/risk", lambda c: c.portfolio_risk()),
    ("cmc/capabilities", lambda c: c.cmc_capabilities()),
]


def _classify(body: Optional[Dict[str, Any]], error: Optional[Exception]) -> Tuple[str, str]:
    """A transport/decode error is FAIL, an ``error`` field is WARN (reachable
    but needs a prior run), otherwise PASS."""
    if error is not None or body is None:
        return "fail", str(error) if error else "no response"
    if "error" in body:
        return "warn", str(body["error"])
    return "pass", ""


def cmd_smoke(client: GuardrailClient, as_json: bool, base: str) -> int:
    results: List[Dict[str, str]] = []
    for name, call in SMOKE_CHECKS:
        body: Optional[Dict[str, Any]] = None
        error: Optional[Exception] = None
        try:
            body = call(client)
        except Exception as exc:  # noqa: BLE001 - any failure is a FAIL outcome
            error = exc
        outcome, detail = _classify(body, error)
        results.append({"name": name, "outcome": outcome, "detail": detail})

    fails = sum(1 for r in results if r["outcome"] == "fail")

    if as_json:
        _print_json({"base": base, "fails": fails, "results": results})
        return EXIT_OK if fails == 0 else EXIT_SMOKE_FAIL

    print(f"quant API smoke against {base}")
    for r in results:
        tag = r["outcome"].upper().ljust(4)
        suffix = f"  ({r['detail']})" if r["detail"] else ""
        print(f"  [{tag}] {r['name'].ljust(20)}{suffix}")
    print()
    if fails == 0:
        print("OK — all quant endpoints responded with valid JSON")
    else:
        print(f"FAILED — {fails} endpoint(s) did not respond correctly")
    return EXIT_OK if fails == 0 else EXIT_SMOKE_FAIL


def _try(fn: Callable[[], Dict[str, Any]]) -> Optional[Dict[str, Any]]:
    """Call an SDK accessor, returning None on any API error (offline-safe)."""
    try:
        return fn()
    except GuardrailApiError:
        return None


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="guardrail",
        description="Operator CLI for the Guardrail Alpha API.",
    )
    parser.add_argument(
        "--base",
        default=None,
        help="API base URL (default $GUARDRAIL_BASE_URL or %s)" % DEFAULT_BASE_URL,
    )
    parser.add_argument(
        "--json", action="store_true", help="emit JSON instead of a table"
    )
    parser.add_argument(
        "command",
        nargs="?",
        default="help",
        choices=["status", "regime", "smoke", "help"],
        help="subcommand to run",
    )
    return parser


def main(argv: Optional[List[str]] = None) -> int:
    parser = build_parser()
    try:
        args = parser.parse_args(argv)
    except SystemExit as exc:  # argparse exits 2 on usage error
        return int(exc.code) if isinstance(exc.code, int) else EXIT_USAGE

    if args.command == "help":
        parser.print_help()
        return EXIT_OK

    base = _resolve_base(args.base)
    client = GuardrailClient(base_url=base, timeout=REQUEST_TIMEOUT_S)

    if args.command == "status":
        return cmd_status(client, args.json)
    if args.command == "regime":
        return cmd_regime(client, args.json)
    if args.command == "smoke":
        return cmd_smoke(client, args.json, base)

    parser.print_help()
    return EXIT_USAGE


if __name__ == "__main__":
    sys.exit(main())
