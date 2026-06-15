#!/usr/bin/env python3
"""Independent, offline proof verifier for the Guardrail BNB AI-Agent.

This is a third-party verification tool. It takes a proof JSON document (as
produced by the agent's `/proof` HTTP route or the on-disk `data/run_report.json`)
and *independently re-derives* the cryptographic commitments the agent claims,
comparing them to the claimed values. It also validates the competition contract
address and BscScan / explorer URL formats. Nothing here trusts the agent: every
check recomputes the value from first principles using only the Python standard
library, so it runs fully offline with no third-party dependencies and no network
or chain access.

How the agent computes its commitments (mirrored exactly here):

  * agent_id     = sha256( name + "\\x00" + wallet )                  (lowercase hex)
                   -- see crates/bnb-agent/src/identity.rs
  * policy_hash  = sha256( raw bytes of the policy file )             (lowercase hex)
                   -- see crates/agent-runtime/src/runtime.rs (sha256_hex_str(policy_raw))
  * report_hash  = sha256( compact JSON of the report "core" object ) (lowercase hex)
                   -- core = {run_id, cycles, final_nav_usd, total_drawdown_pct, events}
                   -- see crates/agent-runtime/src/runtime.rs

Exit code is 0 when every applicable check PASSES, and non-zero otherwise, so the
tool can be wired into CI or a shell gate.

Usage:
    python3 verify.py [PROOF_JSON] [--policy-file PATH] [--strict] [--json]

If PROOF_JSON is omitted, the tool looks for ../../data/run_report.json relative
to this file, then falls back to the bundled sample_proof.json fixture.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from typing import Any, Optional

# ---------------------------------------------------------------------------
# Constants mirrored from the Rust workspace (read-only references).
# ---------------------------------------------------------------------------

# crates/bnb-agent/src/proof.rs :: BSCSCAN_BASE_URL
BSCSCAN_BASE_URL = "https://bscscan.com"

# apps/guardrail-api/src/compete.rs :: COMPETITION_CONTRACT / *_BSCTRACE
COMPETITION_CONTRACT = "0x212c61b9b72c95d95bf29cf032f5e5635629aed5"
COMPETITION_CONTRACT_BSCTRACE = (
    "https://bsctrace.com/address/0x212c61b9b72c95d95bf29cf032f5e5635629aed5"
)

# crates/agent-runtime/src/runtime.rs :: report_hash core field order.
REPORT_CORE_FIELDS = ("run_id", "cycles", "final_nav_usd", "total_drawdown_pct", "events")

# Candidate policy files whose sha256 may match a claimed policy_hash.
# apps/guardrail-api/src/routes/mod.rs :: PRODUCTION_POLICY_PATH / PAPER_POLICY_PATH
DEFAULT_POLICY_CANDIDATES = (
    "configs/risk_policy.paper.json",
    "configs/risk_policy.production.json",
)

# An EVM address: 0x followed by hex characters. Canonical EVM addresses are
# exactly 40 hex chars; this repo's paper/demo wallet is a 41-char vanity
# placeholder (0xA9e5C0FfEe...A1b2C3), so we accept 40-42 as a format sanity
# check rather than enforcing a strict 20-byte length on demo data.
ADDRESS_RE = re.compile(r"^0x[0-9a-fA-F]{40,42}$")
# A strictly canonical 20-byte EVM address (exactly 40 hex chars).
CANONICAL_ADDRESS_RE = re.compile(r"^0x[0-9a-fA-F]{40}$")
# A 32-byte hex transaction hash: 0x followed by exactly 64 hex characters.
TX_HASH_RE = re.compile(r"^0x[0-9a-fA-F]{64}$")
# A SHA-256 hex digest: exactly 64 hex characters (no 0x prefix).
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


# ---------------------------------------------------------------------------
# Pure helpers.
# ---------------------------------------------------------------------------


def sha256_hex(data: bytes) -> str:
    """Lowercase hex SHA-256 of raw bytes (mirrors bnb_agent::sha256_hex)."""
    return hashlib.sha256(data).hexdigest()


def sha256_hex_str(text: str) -> str:
    """Lowercase hex SHA-256 of a UTF-8 string (mirrors sha256_hex_str)."""
    return sha256_hex(text.encode("utf-8"))


def agent_id_for(name: str, wallet: str) -> str:
    """Re-derive agent_id = sha256(name + 0x00 + wallet).

    Mirrors crates/bnb-agent/src/identity.rs::agent_id which joins the two
    fields with a single NUL byte to avoid boundary-collision ambiguity.
    """
    preimage = name.encode("utf-8") + b"\x00" + wallet.encode("utf-8")
    return sha256_hex(preimage)


def report_hash_for(core_source: dict[str, Any]) -> Optional[str]:
    """Re-derive report_hash from the report's core fields.

    The Rust agent builds `core = json!({run_id, cycles, final_nav_usd,
    total_drawdown_pct, events})` and hashes `core.to_string()`. serde_json's
    `to_string` emits compact JSON preserving insertion order, so we reproduce
    that exact byte sequence here. Returns None if any required field is absent.
    """
    if any(field not in core_source for field in REPORT_CORE_FIELDS):
        return None
    ordered = {field: core_source[field] for field in REPORT_CORE_FIELDS}
    # serde_json compact form: no spaces after separators.
    canonical = json.dumps(ordered, separators=(",", ":"), ensure_ascii=False)
    return sha256_hex_str(canonical)


def repo_root_from_here() -> str:
    """Return the workspace root assuming this file lives at clients/proof-verifier/."""
    here = os.path.dirname(os.path.abspath(__file__))
    return os.path.normpath(os.path.join(here, "..", ".."))


# ---------------------------------------------------------------------------
# Check result model (immutable records appended to a list).
# ---------------------------------------------------------------------------


def make_check(name: str, ok: Optional[bool], detail: str) -> dict[str, Any]:
    """Build one immutable check record. `ok=None` marks a skipped check."""
    return {"name": name, "ok": ok, "detail": detail}


# ---------------------------------------------------------------------------
# Proof extraction. Handles both the /proof envelope and a bare run_report.
# ---------------------------------------------------------------------------


def extract_claims(proof: dict[str, Any]) -> dict[str, Any]:
    """Normalize the many proof shapes into a flat claims dict.

    Supported inputs:
      * /proof envelope: {agent, registration_tx, latest_report{...}, run_report{...}}
      * AgentReportPublished summary: {agent_id, wallet_address, policy_hash,
        report_hash, address_url, ...}
      * bare data/run_report.json: {wallet_address, policy_hash, run_id, ...}
    """
    # Prefer the richest commitment-bearing object available.
    latest = proof.get("latest_report")
    run_report = proof.get("run_report")
    summary = latest if isinstance(latest, dict) else {}
    report = run_report if isinstance(run_report, dict) else {}

    # If the top-level doc *is* a bare report (has policy_hash but no nesting),
    # treat it as both summary and report.
    if not summary and not report and "policy_hash" in proof:
        summary = proof
        report = proof

    def pick(key: str) -> Any:
        if key in summary:
            return summary[key]
        if key in report:
            return report[key]
        return proof.get(key)

    return {
        "agent": proof.get("agent") or pick("agent") or pick("name"),
        "agent_id": pick("agent_id"),
        "wallet_address": pick("wallet_address") or pick("wallet"),
        "policy_hash": pick("policy_hash"),
        "report_hash": pick("report_hash"),
        "address_url": pick("address_url"),
        "registration_tx": proof.get("registration_tx") or pick("registration_tx"),
        "registration_tx_url": pick("registration_tx_url"),
        # Source object used to re-derive report_hash (core fields live here).
        "report_core_source": summary or report or proof,
        # The on-disk run report carries name/run_id used for cross-checks.
        "run_report": report,
    }


# ---------------------------------------------------------------------------
# Individual verification stages.
# ---------------------------------------------------------------------------


def verify_policy_hash(
    claims: dict[str, Any], policy_file: Optional[str], root: str
) -> dict[str, Any]:
    claimed = claims.get("policy_hash")
    if not claimed:
        return make_check("policy_hash", None, "no policy_hash claimed in proof; skipped")
    if not SHA256_RE.match(str(claimed)):
        return make_check(
            "policy_hash",
            False,
            f"claimed policy_hash is not a 64-char lowercase hex digest: {claimed!r}",
        )

    candidates = [policy_file] if policy_file else [
        os.path.join(root, rel) for rel in DEFAULT_POLICY_CANDIDATES
    ]
    tried: list[str] = []
    for path in candidates:
        if not path or not os.path.isfile(path):
            if path:
                tried.append(f"{path} (missing)")
            continue
        with open(path, "rb") as handle:
            recomputed = sha256_hex(handle.read())
        tried.append(path)
        if recomputed == claimed:
            return make_check(
                "policy_hash",
                True,
                f"recomputed sha256 of {os.path.relpath(path, root)} matches claimed {claimed}",
            )

    if not tried:
        return make_check(
            "policy_hash",
            None,
            "no policy file available to recompute against; skipped "
            f"(looked for: {', '.join(DEFAULT_POLICY_CANDIDATES)})",
        )
    return make_check(
        "policy_hash",
        False,
        f"claimed {claimed} did not match sha256 of any candidate policy file "
        f"(tried: {', '.join(tried)})",
    )


def verify_report_hash(claims: dict[str, Any]) -> dict[str, Any]:
    claimed = claims.get("report_hash")
    if not claimed:
        return make_check(
            "report_hash",
            None,
            "no report_hash claimed (bare run reports omit it); skipped",
        )
    if not SHA256_RE.match(str(claimed)):
        return make_check(
            "report_hash",
            False,
            f"claimed report_hash is not a 64-char lowercase hex digest: {claimed!r}",
        )
    source = claims.get("report_core_source") or {}
    recomputed = report_hash_for(source)
    if recomputed is None:
        missing = [f for f in REPORT_CORE_FIELDS if f not in source]
        return make_check(
            "report_hash",
            False,
            "cannot re-derive report_hash: proof is missing core field(s) "
            f"{missing} required by the agent's hashing (run with the /proof "
            "envelope's latest_report, which carries them)",
        )
    if recomputed == claimed:
        return make_check(
            "report_hash",
            True,
            f"recomputed sha256 over {{{', '.join(REPORT_CORE_FIELDS)}}} matches claimed {claimed}",
        )
    return make_check(
        "report_hash",
        False,
        f"recomputed {recomputed} != claimed {claimed}",
    )


def verify_agent_id(claims: dict[str, Any]) -> dict[str, Any]:
    claimed = claims.get("agent_id")
    name = claims.get("agent")
    wallet = claims.get("wallet_address")
    if not claimed:
        return make_check("agent_id", None, "no agent_id claimed in proof; skipped")
    if not name or not wallet:
        return make_check(
            "agent_id",
            None,
            "cannot re-derive agent_id without both agent name and wallet; skipped",
        )
    recomputed = agent_id_for(str(name), str(wallet))
    if recomputed == claimed:
        return make_check(
            "agent_id",
            True,
            f"recomputed sha256(name\\x00wallet) matches claimed {claimed}",
        )
    return make_check(
        "agent_id",
        False,
        f"recomputed {recomputed} != claimed {claimed} (name={name!r}, wallet={wallet!r})",
    )


def verify_wallet(claims: dict[str, Any]) -> dict[str, Any]:
    wallet = claims.get("wallet_address")
    if not wallet:
        return make_check("wallet_address", False, "proof carries no wallet_address")
    wallet = str(wallet)
    if CANONICAL_ADDRESS_RE.match(wallet):
        return make_check("wallet_address", True, f"valid 20-byte EVM address: {wallet}")
    if ADDRESS_RE.match(wallet):
        return make_check(
            "wallet_address",
            True,
            f"0x-prefixed hex address (demo/vanity placeholder length): {wallet}",
        )
    return make_check(
        "wallet_address",
        False,
        f"wallet_address is not a 0x-prefixed hex EVM address: {wallet!r}",
    )


def verify_address_url(claims: dict[str, Any]) -> dict[str, Any]:
    wallet = claims.get("wallet_address")
    url = claims.get("address_url")
    if not url:
        return make_check(
            "address_url",
            None,
            "no address_url claimed (bare run reports omit it); skipped",
        )
    if not wallet:
        return make_check(
            "address_url", False, "address_url present but wallet_address missing"
        )
    expected = f"{BSCSCAN_BASE_URL}/address/{wallet}"
    if url == expected:
        return make_check("address_url", True, f"BscScan address URL well-formed: {url}")
    return make_check(
        "address_url",
        False,
        f"address_url {url!r} != expected {expected!r}",
    )


def verify_registration_tx(claims: dict[str, Any]) -> dict[str, Any]:
    tx = claims.get("registration_tx")
    if not tx:
        return make_check(
            "registration_tx",
            None,
            "no registration_tx anchored yet (optional, set out-of-band); skipped",
        )
    if not TX_HASH_RE.match(str(tx)):
        return make_check(
            "registration_tx",
            False,
            f"registration_tx is not a 0x + 64-hex tx hash: {tx!r}",
        )
    tx_url = claims.get("registration_tx_url")
    if tx_url:
        expected = f"{BSCSCAN_BASE_URL}/tx/{tx}"
        if tx_url != expected:
            return make_check(
                "registration_tx",
                False,
                f"registration_tx_url {tx_url!r} != expected {expected!r}",
            )
    return make_check("registration_tx", True, f"valid tx hash format: {tx}")


def verify_competition_contract() -> list[dict[str, Any]]:
    """Validate the competition contract address and explorer URL formats.

    These constants are fixed by the competition (mirrored from
    apps/guardrail-api/src/compete.rs). We assert the address is a well-formed
    EVM address and that the published BscTrace explorer URL embeds it exactly.
    """
    checks: list[dict[str, Any]] = []
    addr_ok = bool(ADDRESS_RE.match(COMPETITION_CONTRACT))
    checks.append(
        make_check(
            "competition_contract_format",
            addr_ok,
            f"competition contract is a valid EVM address: {COMPETITION_CONTRACT}"
            if addr_ok
            else f"competition contract is malformed: {COMPETITION_CONTRACT}",
        )
    )
    expected_explorer = f"https://bsctrace.com/address/{COMPETITION_CONTRACT}"
    explorer_ok = COMPETITION_CONTRACT_BSCTRACE == expected_explorer
    checks.append(
        make_check(
            "competition_contract_explorer_url",
            explorer_ok,
            f"explorer URL embeds the contract: {COMPETITION_CONTRACT_BSCTRACE}"
            if explorer_ok
            else f"explorer URL {COMPETITION_CONTRACT_BSCTRACE!r} does not embed "
            f"{COMPETITION_CONTRACT!r}",
        )
    )
    return checks


def run_all_checks(
    proof: dict[str, Any], policy_file: Optional[str], root: str
) -> list[dict[str, Any]]:
    claims = extract_claims(proof)
    checks: list[dict[str, Any]] = [
        verify_wallet(claims),
        verify_policy_hash(claims, policy_file, root),
        verify_report_hash(claims),
        verify_agent_id(claims),
        verify_address_url(claims),
        verify_registration_tx(claims),
    ]
    checks.extend(verify_competition_contract())
    return checks


# ---------------------------------------------------------------------------
# Reporting.
# ---------------------------------------------------------------------------


def render_report(source_path: str, checks: list[dict[str, Any]]) -> str:
    lines = [
        "============================================================",
        " Guardrail BNB AI-Agent — Independent Proof Verification",
        "============================================================",
        f" proof source : {source_path}",
        "",
    ]
    for check in checks:
        if check["ok"] is True:
            mark = "PASS"
        elif check["ok"] is False:
            mark = "FAIL"
        else:
            mark = "SKIP"
        lines.append(f" [{mark}] {check['name']}")
        lines.append(f"        {check['detail']}")
    passed = sum(1 for c in checks if c["ok"] is True)
    failed = sum(1 for c in checks if c["ok"] is False)
    skipped = sum(1 for c in checks if c["ok"] is None)
    lines.append("")
    lines.append("------------------------------------------------------------")
    overall = "PASS" if failed == 0 else "FAIL"
    lines.append(
        f" RESULT: {overall}  ({passed} passed, {failed} failed, {skipped} skipped)"
    )
    lines.append("============================================================")
    return "\n".join(lines)


def resolve_default_proof(root: str) -> tuple[dict[str, Any], str]:
    """Pick the proof to verify: real run report if present, else the fixture."""
    run_report = os.path.join(root, "data", "run_report.json")
    if os.path.isfile(run_report):
        with open(run_report, "r", encoding="utf-8") as handle:
            return json.load(handle), run_report
    fixture = os.path.join(os.path.dirname(os.path.abspath(__file__)), "sample_proof.json")
    with open(fixture, "r", encoding="utf-8") as handle:
        return json.load(handle), fixture


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        description="Independently verify a Guardrail BNB AI-Agent proof (offline)."
    )
    parser.add_argument(
        "proof",
        nargs="?",
        help="Path to a proof JSON (from /proof or data/run_report.json). "
        "Defaults to data/run_report.json, then the bundled sample fixture.",
    )
    parser.add_argument(
        "--policy-file",
        help="Explicit policy file to recompute policy_hash against. "
        "Defaults to configs/risk_policy.paper.json then .production.json.",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Treat SKIPPED checks as failures (require every commitment present).",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit machine-readable JSON instead of the text report.",
    )
    args = parser.parse_args(argv)

    root = repo_root_from_here()

    try:
        if args.proof:
            with open(args.proof, "r", encoding="utf-8") as handle:
                proof = json.load(handle)
            source_path = args.proof
        else:
            proof, source_path = resolve_default_proof(root)
    except FileNotFoundError as err:
        print(f"error: proof file not found: {err}", file=sys.stderr)
        return 2
    except json.JSONDecodeError as err:
        print(f"error: proof file is not valid JSON: {err}", file=sys.stderr)
        return 2

    if not isinstance(proof, dict):
        print("error: proof JSON must be an object", file=sys.stderr)
        return 2

    policy_file = args.policy_file
    if policy_file and not os.path.isabs(policy_file):
        # Resolve relative paths against cwd first, then repo root.
        if not os.path.isfile(policy_file):
            candidate = os.path.join(root, policy_file)
            if os.path.isfile(candidate):
                policy_file = candidate

    checks = run_all_checks(proof, policy_file, root)

    failed = sum(1 for c in checks if c["ok"] is False)
    skipped = sum(1 for c in checks if c["ok"] is None)

    if args.json:
        payload = {
            "source": source_path,
            "checks": checks,
            "result": "PASS" if failed == 0 and not (args.strict and skipped) else "FAIL",
        }
        print(json.dumps(payload, indent=2))
    else:
        print(render_report(source_path, checks))

    if failed > 0:
        return 1
    if args.strict and skipped > 0:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
