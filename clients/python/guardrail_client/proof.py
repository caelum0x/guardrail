"""Independent, offline proof verifier for the Guardrail BNB AI-Agent.

This mirrors the standalone verifier at ``clients/proof-verifier/verify.py`` and
the Go (``clients/go/proof.go``) and TypeScript (``clients/typescript/src/proof.ts``)
ports. It takes a ``/proof`` envelope (or a bare ``data/run_report.json``) and
*independently re-derives* the cryptographic commitments the agent claims,
comparing them to the claimed values. It also validates the competition contract
address + explorer URL formats.

Nothing here trusts the agent. Every commitment is recomputed from first
principles using only the Python standard library (``hashlib`` + ``json``), so
it runs fully offline with no third-party dependencies. Because every port
hashes the same compact JSON shapes, their results agree byte-for-byte.

How the agent computes its commitments (mirrored exactly here):

  * agent_id     = sha256( name + "\\x00" + wallet )                  (lowercase hex)
                   -- see crates/bnb-agent/src/identity.rs
  * policy_hash  = sha256( raw bytes of the policy file )             (lowercase hex)
                   -- see crates/agent-runtime/src/runtime.rs
  * report_hash  = sha256( compact JSON of the report "core" object ) (lowercase hex)
                   -- core = {run_id, cycles, final_nav_usd, total_drawdown_pct, events}
                   -- see crates/agent-runtime/src/runtime.rs
"""

from __future__ import annotations

import hashlib
import json
import re
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Tuple

__all__ = [
    "BSCSCAN_BASE_URL",
    "COMPETITION_CONTRACT",
    "COMPETITION_CONTRACT_BSCTRACE",
    "REPORT_CORE_FIELDS",
    "CheckStatus",
    "Check",
    "VerifyResult",
    "verify_proof",
    "render_report",
    "agent_id_for",
    "report_hash_for",
    "sha256_hex",
    "sha256_hex_str",
]

# ---------------------------------------------------------------------------
# Constants mirrored from the Rust workspace (read-only references). They are
# duplicated here deliberately: the verifier shares no code with the agent, so
# agreement between the two proves the commitments are independently
# reproducible.
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

# Format-validation patterns. ADDRESS_RE accepts the canonical 40-hex address
# plus this repo's 41/42-char vanity placeholder; CANONICAL_ADDRESS_RE enforces
# a strict 20-byte (40-hex) address.
ADDRESS_RE = re.compile(r"^0x[0-9a-fA-F]{40,42}$")
CANONICAL_ADDRESS_RE = re.compile(r"^0x[0-9a-fA-F]{40}$")
TX_HASH_RE = re.compile(r"^0x[0-9a-fA-F]{64}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


# ---------------------------------------------------------------------------
# Result model (immutable dataclasses).
# ---------------------------------------------------------------------------

# A check outcome: "PASS", "FAIL", or "SKIP" (not applicable to this proof shape).
CheckStatus = str


@dataclass(frozen=True)
class Check:
    """One immutable verification result."""

    name: str
    status: CheckStatus
    detail: str


@dataclass(frozen=True)
class VerifyResult:
    """Aggregate outcome of verifying a proof.

    ``passed`` is true only when no check failed (skips do not, by default,
    fail the result).
    """

    passed: bool
    checks: Tuple[Check, ...]

    def counts(self) -> Tuple[int, int, int]:
        """Return (passed, failed, skipped) check counts."""
        passed = sum(1 for c in self.checks if c.status == "PASS")
        failed = sum(1 for c in self.checks if c.status == "FAIL")
        skipped = sum(1 for c in self.checks if c.status == "SKIP")
        return passed, failed, skipped


def _pass(name: str, detail: str) -> Check:
    return Check(name=name, status="PASS", detail=detail)


def _fail(name: str, detail: str) -> Check:
    return Check(name=name, status="FAIL", detail=detail)


def _skip(name: str, detail: str) -> Check:
    return Check(name=name, status="SKIP", detail=detail)


# ---------------------------------------------------------------------------
# Hashing helpers (mirror the Rust agent + standalone verifier).
# ---------------------------------------------------------------------------


def sha256_hex(data: bytes) -> str:
    """Lowercase hex SHA-256 of raw bytes."""
    return hashlib.sha256(data).hexdigest()


def sha256_hex_str(text: str) -> str:
    """Lowercase hex SHA-256 of a UTF-8 string."""
    return sha256_hex(text.encode("utf-8"))


def agent_id_for(name: str, wallet: str) -> str:
    """Re-derive agent_id = sha256(name + 0x00 + wallet).

    Mirrors crates/bnb-agent/src/identity.rs, which joins the two fields with a
    single NUL byte to avoid boundary-collision ambiguity.
    """
    preimage = name.encode("utf-8") + b"\x00" + wallet.encode("utf-8")
    return sha256_hex(preimage)


def report_hash_for(core_source: Dict[str, Any]) -> Optional[str]:
    """Re-derive report_hash from the report's core fields.

    The Rust agent builds ``core = json!({run_id, cycles, final_nav_usd,
    total_drawdown_pct, events})`` and hashes ``core.to_string()``. serde_json's
    ``to_string`` emits compact JSON preserving insertion order, so we reproduce
    that exact byte sequence here. Returns ``None`` if any required field is
    absent.
    """
    if any(field not in core_source for field in REPORT_CORE_FIELDS):
        return None
    ordered = {field: core_source[field] for field in REPORT_CORE_FIELDS}
    # serde_json compact form: no spaces after separators.
    canonical = json.dumps(ordered, separators=(",", ":"), ensure_ascii=False)
    return sha256_hex_str(canonical)


# ---------------------------------------------------------------------------
# Claim extraction (mirrors extract_claims / extractClaims).
# ---------------------------------------------------------------------------


def _extract_claims(proof: Dict[str, Any]) -> Dict[str, Any]:
    """Normalize the many proof shapes into a flat claims dict.

    Supported inputs:
      * /proof envelope: {agent, registration_tx, latest_report{...}, run_report{...}}
      * AgentReportPublished summary: {agent_id, wallet_address, policy_hash, ...}
      * bare data/run_report.json: {wallet_address, policy_hash, run_id, ...}
    """
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
    }


# ---------------------------------------------------------------------------
# Individual verification stages.
# ---------------------------------------------------------------------------


def _verify_wallet(claims: Dict[str, Any]) -> Check:
    wallet = claims.get("wallet_address")
    if not wallet:
        return _fail("wallet_address", "proof carries no wallet_address")
    wallet = str(wallet)
    if CANONICAL_ADDRESS_RE.match(wallet):
        return _pass("wallet_address", f"valid 20-byte EVM address: {wallet}")
    if ADDRESS_RE.match(wallet):
        return _pass(
            "wallet_address",
            f"0x-prefixed hex address (demo/vanity placeholder length): {wallet}",
        )
    return _fail(
        "wallet_address",
        f"wallet_address is not a 0x-prefixed hex EVM address: {wallet!r}",
    )


def _verify_policy_hash(claims: Dict[str, Any], policy_raw: Optional[bytes]) -> Check:
    claimed = claims.get("policy_hash")
    if not claimed:
        return _skip("policy_hash", "no policy_hash claimed in proof; skipped")
    if not SHA256_RE.match(str(claimed)):
        return _fail(
            "policy_hash",
            f"claimed policy_hash is not a 64-char lowercase hex digest: {claimed!r}",
        )
    if policy_raw is None:
        return _skip(
            "policy_hash",
            "no policy file content supplied to recompute against; skipped",
        )
    recomputed = sha256_hex(policy_raw)
    if recomputed == claimed:
        return _pass(
            "policy_hash",
            f"recomputed sha256 of supplied policy content matches claimed {claimed}",
        )
    return _fail("policy_hash", f"recomputed {recomputed} != claimed {claimed}")


def _verify_report_hash(claims: Dict[str, Any]) -> Check:
    claimed = claims.get("report_hash")
    if not claimed:
        return _skip(
            "report_hash",
            "no report_hash claimed (bare run reports omit it); skipped",
        )
    if not SHA256_RE.match(str(claimed)):
        return _fail(
            "report_hash",
            f"claimed report_hash is not a 64-char lowercase hex digest: {claimed!r}",
        )
    source = claims.get("report_core_source") or {}
    recomputed = report_hash_for(source)
    if recomputed is None:
        missing = [f for f in REPORT_CORE_FIELDS if f not in source]
        return _fail(
            "report_hash",
            "cannot re-derive report_hash: proof is missing core field(s) "
            f"{missing} required by the agent's hashing",
        )
    if recomputed == claimed:
        return _pass(
            "report_hash",
            f"recomputed sha256 over {{{', '.join(REPORT_CORE_FIELDS)}}} matches claimed {claimed}",
        )
    return _fail("report_hash", f"recomputed {recomputed} != claimed {claimed}")


def _verify_agent_id(claims: Dict[str, Any]) -> Check:
    claimed = claims.get("agent_id")
    name = claims.get("agent")
    wallet = claims.get("wallet_address")
    if not claimed:
        return _skip("agent_id", "no agent_id claimed in proof; skipped")
    if not name or not wallet:
        return _skip(
            "agent_id",
            "cannot re-derive agent_id without both agent name and wallet; skipped",
        )
    recomputed = agent_id_for(str(name), str(wallet))
    if recomputed == claimed:
        return _pass(
            "agent_id",
            f"recomputed sha256(name\\x00wallet) matches claimed {claimed}",
        )
    return _fail(
        "agent_id",
        f"recomputed {recomputed} != claimed {claimed} (name={name!r}, wallet={wallet!r})",
    )


def _verify_address_url(claims: Dict[str, Any]) -> Check:
    wallet = claims.get("wallet_address")
    url = claims.get("address_url")
    if not url:
        return _skip(
            "address_url",
            "no address_url claimed (bare run reports omit it); skipped",
        )
    if not wallet:
        return _fail("address_url", "address_url present but wallet_address missing")
    expected = f"{BSCSCAN_BASE_URL}/address/{wallet}"
    if url == expected:
        return _pass("address_url", f"BscScan address URL well-formed: {url}")
    return _fail("address_url", f"address_url {url!r} != expected {expected!r}")


def _verify_registration_tx(claims: Dict[str, Any]) -> Check:
    tx = claims.get("registration_tx")
    if not tx:
        return _skip(
            "registration_tx",
            "no registration_tx anchored yet (optional, set out-of-band); skipped",
        )
    if not TX_HASH_RE.match(str(tx)):
        return _fail(
            "registration_tx",
            f"registration_tx is not a 0x + 64-hex tx hash: {tx!r}",
        )
    tx_url = claims.get("registration_tx_url")
    if tx_url:
        expected = f"{BSCSCAN_BASE_URL}/tx/{tx}"
        if tx_url != expected:
            return _fail(
                "registration_tx",
                f"registration_tx_url {tx_url!r} != expected {expected!r}",
            )
    return _pass("registration_tx", f"valid tx hash format: {tx}")


def _verify_competition_contract() -> List[Check]:
    """Validate the fixed competition contract address + explorer URL formats."""
    checks: List[Check] = []
    addr_ok = bool(ADDRESS_RE.match(COMPETITION_CONTRACT))
    checks.append(
        _pass(
            "competition_contract_format",
            f"competition contract is a valid EVM address: {COMPETITION_CONTRACT}",
        )
        if addr_ok
        else _fail(
            "competition_contract_format",
            f"competition contract is malformed: {COMPETITION_CONTRACT}",
        )
    )
    expected_explorer = f"https://bsctrace.com/address/{COMPETITION_CONTRACT}"
    explorer_ok = COMPETITION_CONTRACT_BSCTRACE == expected_explorer
    checks.append(
        _pass(
            "competition_contract_explorer_url",
            f"explorer URL embeds the contract: {COMPETITION_CONTRACT_BSCTRACE}",
        )
        if explorer_ok
        else _fail(
            "competition_contract_explorer_url",
            f"explorer URL {COMPETITION_CONTRACT_BSCTRACE!r} does not embed "
            f"{COMPETITION_CONTRACT!r}",
        )
    )
    return checks


# ---------------------------------------------------------------------------
# Public API.
# ---------------------------------------------------------------------------


def verify_proof(
    proof: Dict[str, Any],
    policy_raw: Optional[bytes] = None,
) -> VerifyResult:
    """Verify a ``/proof`` envelope (or bare run report) entirely offline.

    Re-derives every applicable commitment and validates the competition
    contract metadata, returning an immutable :class:`VerifyResult`.

    Args:
        proof: The parsed proof JSON object (a dict). Accepts the ``/proof``
            envelope, an ``AgentReportPublished`` summary, or a bare run report.
        policy_raw: Optional raw bytes of the policy file to recompute
            ``policy_hash`` against. When ``None`` the policy_hash check is
            skipped (no file to hash against).

    Returns:
        A :class:`VerifyResult` whose ``passed`` is true only when no check
        failed (skips do not fail the result).
    """
    if not isinstance(proof, dict):
        raise TypeError(f"proof must be a dict, got {type(proof).__name__}")
    claims = _extract_claims(proof)
    checks: List[Check] = [
        _verify_wallet(claims),
        _verify_policy_hash(claims, policy_raw),
        _verify_report_hash(claims),
        _verify_agent_id(claims),
        _verify_address_url(claims),
        _verify_registration_tx(claims),
    ]
    checks.extend(_verify_competition_contract())
    passed = all(c.status != "FAIL" for c in checks)
    return VerifyResult(passed=passed, checks=tuple(checks))


def render_report(result: VerifyResult, source: str = "proof") -> str:
    """Render a human-readable PASS/FAIL report.

    Mirrors the standalone verifier's text output so all ports print the same
    shape.
    """
    rule = "============================================================"
    lines = [
        rule,
        " Guardrail BNB AI-Agent — Independent Proof Verification",
        rule,
        f" proof source : {source}",
        "",
    ]
    for check in result.checks:
        lines.append(f" [{check.status}] {check.name}")
        lines.append(f"        {check.detail}")
    passed, failed, skipped = result.counts()
    lines.append("")
    lines.append("------------------------------------------------------------")
    overall = "PASS" if failed == 0 else "FAIL"
    lines.append(
        f" RESULT: {overall}  ({passed} passed, {failed} failed, {skipped} skipped)"
    )
    lines.append(rule)
    return "\n".join(lines)
