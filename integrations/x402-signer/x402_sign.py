#!/usr/bin/env python3
"""Real x402 payment signer — the self-custody signing backend for guardrail.

Replaces the deterministic mock signer in `crates/twak-client/src/x402.rs` with
a genuine EIP-712 / EIP-3009 `TransferWithAuthorization` signature produced by
the BNB Agent SDK's policy-gated `X402Signer`. Keys never leave the local
encrypted keystore (`~/.bnbagent/wallets/`, Keystore V3); this process signs and
prints the `X-PAYMENT` envelope, nothing else.

Protocol (mirrors `integrations/bnbagent-sdk/examples/x402_buyer_demo.py`):
  stdin  ← JSON challenge: an x402 v2 body `{accepts:[...]}`, a single `accept`
           entry, or a normalized `{asset, payTo, amount, network|chainId,
           name, version}` object.
  args   ← --expected-to (the payee the CALLER commits to, sourced independently
           of the challenge body — this is the anti-MITM guard) and optional
           --chain-id / --caps-file.
  stdout → JSON: {"signature","from","envelope","authorization"} (pure JSON;
           all logs go to stderr so stdout stays machine-parseable).

Gating: requires WALLET_PASSWORD (and PRIVATE_KEY on first run to import a key).
Per-call value cap and session budget are read from the caps file (defaults to
`configs/x402/signing_policy.json`) so an over-priced 402 cannot drain the wallet.

Exit codes: 0 ok · 2 bad input · 3 signing refused by policy · 4 config/env.
"""

from __future__ import annotations

import argparse
import base64
import json
import logging
import os
import secrets
import sys
import time
from pathlib import Path
from typing import Any

log = logging.getLogger("x402_sign")

# EIP-712 schema — identical to the production U-token EIP-3009 domain.
_EIP712_DOMAIN_FIELDS = [
    {"name": "name", "type": "string"},
    {"name": "version", "type": "string"},
    {"name": "chainId", "type": "uint256"},
    {"name": "verifyingContract", "type": "address"},
]
_TWA_FIELDS = [
    {"name": "from", "type": "address"},
    {"name": "to", "type": "address"},
    {"name": "value", "type": "uint256"},
    {"name": "validAfter", "type": "uint256"},
    {"name": "validBefore", "type": "uint256"},
    {"name": "nonce", "type": "bytes32"},
]


def _fail(code: int, msg: str) -> int:
    """Log an error to stderr and return an exit code (stdout stays clean)."""
    log.error(msg)
    return code


def _normalize_accept(payload: dict[str, Any]) -> dict[str, Any]:
    """Reduce any accepted challenge shape to a single `accept` dict."""
    if "accepts" in payload and isinstance(payload["accepts"], list) and payload["accepts"]:
        return payload["accepts"][0]
    return payload


def _chain_id(accept: dict[str, Any], override: int | None) -> int:
    """Resolve chainId from --chain-id, an `eip155:NN` network, or a chainId field."""
    if override is not None:
        return override
    net = accept.get("network")
    if isinstance(net, str) and net.startswith("eip155:"):
        return int(net.split(":", 1)[1])
    if "chainId" in accept:
        return int(accept["chainId"])
    return 56  # BSC mainnet default (the competition chain).


def _load_caps(caps_file: Path | None, token: str) -> tuple[dict[str, int], dict[str, int]]:
    """Build per-call and session caps for `token` from the signing policy file."""
    if caps_file is None or not caps_file.exists():
        return {}, {}
    policy = json.loads(caps_file.read_text())
    per_call = policy.get("max_per_call_base_units")
    session = policy.get("session_budget_base_units")
    max_value = {token: int(per_call)} if per_call is not None else {}
    budget = {token: int(session)} if session is not None else {}
    return max_value, budget


def _build_message(from_addr: str, accept: dict[str, Any]) -> dict[str, Any]:
    """Materialize a TransferWithAuthorization message from a 402 accept entry."""
    now = int(time.time())
    timeout = int(accept.get("maxTimeoutSeconds", 300))
    return {
        "from": from_addr,
        "to": accept["payTo"],
        "value": int(accept["amount"]),
        "validAfter": now - 60,
        "validBefore": now + timeout,
        "nonce": "0x" + secrets.token_hex(32),
    }


def _envelope(accept: dict[str, Any], msg: dict[str, Any], signature: str) -> str:
    """Encode the base64(json) X-PAYMENT envelope per x402 v2."""
    env = {
        "x402Version": 2,
        "scheme": accept.get("scheme", "exact"),
        "network": accept.get("network", "eip155:56"),
        "payload": {
            "authorization": {
                "from": msg["from"],
                "to": msg["to"],
                "value": str(msg["value"]),
                "validAfter": str(msg["validAfter"]),
                "validBefore": str(msg["validBefore"]),
                "nonce": msg["nonce"],
            },
            "signature": signature,
        },
    }
    return base64.b64encode(json.dumps(env).encode()).decode()


def main(argv: list[str] | None = None) -> int:
    logging.basicConfig(
        level=logging.INFO, stream=sys.stderr, format="%(levelname)s %(name)s | %(message)s"
    )
    parser = argparse.ArgumentParser(description="Sign an x402 payment via the BNB SDK X402Signer.")
    parser.add_argument(
        "--expected-to",
        required=True,
        help="Payee the caller commits to (independent of the challenge body).",
    )
    parser.add_argument("--chain-id", type=int, default=None, help="Override chainId.")
    parser.add_argument(
        "--caps-file",
        type=Path,
        default=Path("configs/x402/signing_policy.json"),
        help="Signing-policy JSON for per-call / session caps.",
    )
    args = parser.parse_args(argv)

    password = os.environ.get("WALLET_PASSWORD")
    if not password:
        return _fail(4, "WALLET_PASSWORD is required to unlock the signing keystore")

    try:
        raw = sys.stdin.read()
        accept = _normalize_accept(json.loads(raw))
    except (json.JSONDecodeError, ValueError) as e:
        return _fail(2, f"invalid challenge JSON on stdin: {e}")

    token = accept.get("asset")
    if not token or "payTo" not in accept or "amount" not in accept:
        return _fail(2, "challenge must include asset, payTo, and amount")

    chain_id = _chain_id(accept, args.chain_id)
    # Reconstruct the EIP-712 domain. name/version come from the challenge's
    # `extra` (x402 v2) or top-level fields; verifyingContract is the token.
    extra = accept.get("extra", {})
    name = extra.get("name") or accept.get("name")
    version = extra.get("version") or accept.get("version")
    if not name or not version:
        return _fail(2, "challenge missing EIP-712 token name/version (extra.name/version)")

    # Import here so --help works without the SDK installed.
    try:
        from bnbagent import EVMWalletProvider, X402Signer  # noqa: PLC0415
    except ImportError as e:
        return _fail(4, f"bnbagent SDK not installed: {e} (pip install bnbagent)")

    private_key = os.environ.get("PRIVATE_KEY") or None
    try:
        wallet = EVMWalletProvider(password=password, private_key=private_key)
    except Exception as e:  # noqa: BLE001 — surface any keystore error as exit 4
        return _fail(4, f"failed to open wallet keystore: {e}")

    max_value, session_budget = _load_caps(args.caps_file, token)
    signer = X402Signer(wallet, max_value_per_call=max_value, session_budget=session_budget)

    domain = {
        "name": name,
        "version": version,
        "chainId": chain_id,
        "verifyingContract": token,
    }
    types = {"EIP712Domain": _EIP712_DOMAIN_FIELDS, "TransferWithAuthorization": _TWA_FIELDS}
    message = _build_message(wallet.address, accept)

    try:
        signed = signer.sign_payment(
            domain=domain, types=types, message=message, expected_to=args.expected_to
        )
    except Exception as e:  # noqa: BLE001 — policy refusals are an expected outcome
        return _fail(3, f"signing refused by policy: {type(e).__name__}: {e}")

    raw_sig = signed["signature"]
    sig = raw_sig.hex() if hasattr(raw_sig, "hex") and not isinstance(raw_sig, str) else raw_sig
    if not sig.startswith("0x"):
        sig = "0x" + sig

    out = {
        "signature": sig,
        "from": wallet.address,
        "envelope": _envelope(accept, message, sig),
        "authorization": {
            "from": message["from"],
            "to": message["to"],
            "value": str(message["value"]),
            "validAfter": str(message["validAfter"]),
            "validBefore": str(message["validBefore"]),
            "nonce": message["nonce"],
        },
    }
    print(json.dumps(out))
    log.info("x402 payment signed: token=%s value=%s to=%s", token, message["value"], message["to"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
