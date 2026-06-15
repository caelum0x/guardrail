#!/usr/bin/env python3
"""Regime-routed ensemble view -- mirrors the cockpit's ensemble routing.

This stdlib-only demo shows how a client turns the live market *regime* into the
ensemble meta-allocator's per-skill blend, exactly the way the operator cockpit
does:

    1. GET /regime              -> classified regime (risk_on | risk_off | chop | breakout)
    2. load skills/ensemble.json -> per-regime blend weights over the four skills
    3. print the regime-routed blend the ensemble would propose for that regime

The blend weights are read from the committed ``skills/ensemble.json`` (the same
config the Rust/Python ensemble uses), so the view is reproducible offline from
the repository alone. The Rust risk engine remains the sole execution gate; this
is an advisory target, never an order (see docs/ENSEMBLE.md).

Offline-safe: if the API is unreachable, the demo falls back to a clearly
labelled default regime, still prints the ensemble routing from the local
config, and exits 0 (never a stack trace).

Run from the repo root (start the API first for a live regime):

    python3 clients/examples/ensemble_demo.py

Configure the target with GUARDRAIL_BASE_URL (default http://localhost:8080).
"""

from __future__ import annotations

import json
import os
import urllib.error
import urllib.request
from typing import Any, Dict, Optional, Tuple

DEFAULT_BASE_URL = "http://localhost:8080"
DEFAULT_REGIME = "risk_on"

# skills/ensemble.json lives at the repo root: clients/examples/ -> ../../skills
_THIS_DIR = os.path.dirname(os.path.abspath(__file__))
_ENSEMBLE_CONFIG = os.path.normpath(
    os.path.join(_THIS_DIR, "..", "..", "skills", "ensemble.json")
)


def fetch_regime(base_url: str, timeout: float = 8.0) -> Tuple[str, Optional[str]]:
    """Return ``(regime, note)``. ``note`` is set when we fell back to a default.

    Never raises: any transport/parse failure degrades to the default regime
    with an explanatory note so the demo can run with the API down.
    """
    url = f"{base_url.rstrip('/')}/regime"
    try:
        req = urllib.request.Request(url, headers={"Accept": "application/json"})
        with urllib.request.urlopen(req, timeout=timeout) as resp:  # noqa: S310
            payload = json.loads(resp.read().decode("utf-8"))
    except (urllib.error.URLError, OSError, ValueError, TimeoutError) as exc:
        return DEFAULT_REGIME, f"API unreachable ({exc}); using default regime"

    if isinstance(payload, dict) and payload.get("error"):
        return DEFAULT_REGIME, f"API error ({payload['error']}); using default regime"

    regime = payload.get("regime") if isinstance(payload, dict) else None
    if not isinstance(regime, str) or not regime:
        return DEFAULT_REGIME, "regime missing from response; using default regime"
    return regime, None


def load_ensemble_config() -> Optional[Dict[str, Any]]:
    """Load skills/ensemble.json, or None if it cannot be read/parsed."""
    try:
        with open(_ENSEMBLE_CONFIG, encoding="utf-8") as handle:
            data = json.load(handle)
    except (OSError, ValueError):
        return None
    return data if isinstance(data, dict) else None


def routed_blend(config: Dict[str, Any], regime: str) -> Tuple[Optional[Dict[str, Any]], Optional[str]]:
    """Return ``(regime_block, note)`` for the classified regime.

    Falls back to the default regime block if the live regime is not configured.
    """
    regimes = config.get("regimes") if isinstance(config, dict) else None
    if not isinstance(regimes, dict) or not regimes:
        return None, "ensemble.json has no regimes block"

    block = regimes.get(regime)
    if isinstance(block, dict):
        return block, None

    fallback = regimes.get(DEFAULT_REGIME)
    if isinstance(fallback, dict):
        return fallback, f"regime '{regime}' not configured; showing '{DEFAULT_REGIME}'"
    return None, f"regime '{regime}' not configured and no default available"


def skill_label(config: Dict[str, Any], skill_id: str) -> str:
    skills = config.get("skills")
    if isinstance(skills, dict):
        entry = skills.get(skill_id)
        if isinstance(entry, dict) and isinstance(entry.get("label"), str):
            return entry["label"]
    return skill_id


def print_blend(config: Dict[str, Any], regime: str, block: Dict[str, Any]) -> None:
    weights = block.get("weights")
    if not isinstance(weights, dict) or not weights:
        print("  (no blend weights configured for this regime)")
        return

    ordered = sorted(weights.items(), key=lambda kv: float(kv[1]), reverse=True)
    total = sum(float(v) for _, v in ordered)
    reserve = config.get("reserve_symbol", "USDT")

    print(f"  Ensemble blend for regime '{regime}' (weights sum = {total:.2f}):")
    width = max(len(skill_id) for skill_id, _ in ordered)
    for skill_id, weight in ordered:
        pct = float(weight) * 100.0
        bar = "#" * max(1, round(pct / 4)) if pct > 0 else ""
        label = skill_label(config, skill_id)
        print(f"    {skill_id.ljust(width)}  {pct:5.1f}%  {bar}  ({label})")

    rationale = block.get("rationale")
    if isinstance(rationale, str) and rationale:
        print(f"\n  Why this blend: {rationale}")

    print(
        f"\n  Note: this is the advisory target book. Risk (non-reserve) weights are\n"
        f"  renormalized to <= {config.get('max_risk_allocation_pct', 100)} and the remainder is held in one\n"
        f"  {reserve} reserve line. The Rust risk engine is the sole execution gate."
    )


def main() -> int:
    base_url = os.environ.get("GUARDRAIL_BASE_URL", DEFAULT_BASE_URL)
    print(f"Guardrail regime-routed ensemble demo -> {base_url}\n")

    regime, fetch_note = fetch_regime(base_url)
    print(f"[1/2] /regime -> {regime}")
    if fetch_note:
        print(f"      notice: {fetch_note}")

    config = load_ensemble_config()
    if config is None:
        print(
            f"\n[2/2] could not load ensemble config at {_ENSEMBLE_CONFIG}."
            "\n      Nothing to route; exiting cleanly."
        )
        return 0

    block, route_note = routed_blend(config, regime)
    print("\n[2/2] ensemble routing (skills/ensemble.json)")
    if route_note:
        print(f"      notice: {route_note}")
    if block is None:
        print("      no blend available; exiting cleanly.")
        return 0

    print()
    print_blend(config, regime, block)
    print("\nDone.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
