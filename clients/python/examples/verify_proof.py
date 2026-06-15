#!/usr/bin/env python3
"""Verify the bundled sample proof offline and print the report.

Loads ``clients/proof-verifier/sample_proof.json`` (the fixture shared by every
port) and runs the stdlib-only :func:`guardrail_client.verify_proof` against it.
No network access is required. The agent_id and report_hash checks must PASS;
the script asserts that and exits non-zero otherwise.

Run from the repo root (or anywhere):

    python3 clients/python/examples/verify_proof.py
"""

from __future__ import annotations

import json
import os
import sys

# Allow running directly from the source tree without installing.
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from guardrail_client import render_report, verify_proof  # noqa: E402


def _sample_proof_path() -> str:
    """Resolve the bundled fixture at clients/proof-verifier/sample_proof.json."""
    here = os.path.dirname(os.path.abspath(__file__))
    # clients/python/examples -> clients/proof-verifier/sample_proof.json
    return os.path.normpath(
        os.path.join(here, "..", "..", "proof-verifier", "sample_proof.json")
    )


def main() -> int:
    path = _sample_proof_path()
    if not os.path.isfile(path):
        print(f"error: sample proof not found at {path}", file=sys.stderr)
        return 2

    with open(path, "r", encoding="utf-8") as handle:
        proof = json.load(handle)

    result = verify_proof(proof)
    print(render_report(result, source=path))

    # Surface the key cryptographic checks explicitly: these must agree with the
    # Go / TypeScript / standalone Python ports on the shared fixture.
    by_name = {c.name: c for c in result.checks}
    report_hash = by_name.get("report_hash")
    agent_id = by_name.get("agent_id")

    print()
    if report_hash is not None:
        print(f"report_hash : {report_hash.status}")
    if agent_id is not None:
        print(f"agent_id    : {agent_id.status}")

    if report_hash is None or report_hash.status != "PASS":
        print("error: report_hash did not PASS", file=sys.stderr)
        return 1
    if agent_id is None or agent_id.status != "PASS":
        print("error: agent_id did not PASS", file=sys.stderr)
        return 1
    if not result.passed:
        print("error: one or more checks FAILED", file=sys.stderr)
        return 1

    print("\nOK: sample proof verified offline (report_hash + agent_id PASS).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
