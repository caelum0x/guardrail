# Proof Verification — Independently Verifying the Guardrail Agent's Identity

This document explains how **any third party** can independently verify the
Guardrail BNB AI-Agent's identity and report commitments — offline, without
trusting the agent, and without any network or chain access. It is the narrative
companion to the clean-room verifier in
[`clients/proof-verifier/`](../clients/proof-verifier/).

The point is the BNB SDK prize's "verifiable identity" criterion: the agent does
not merely *assert* an identity, it publishes cryptographic commitments that a
stranger can **re-derive from first principles** and confirm.

## What the agent publishes

The agent emits a proof through three equivalent surfaces:

| Surface | Where | Carries |
|---------|-------|---------|
| `/proof` HTTP route | `apps/guardrail-api/src/routes/mod.rs` | agent, registration_tx, `latest_report` (full commitments), `run_report` |
| `AgentReportPublished` event | `crates/agent-runtime/src/runtime.rs` | agent_id, wallet, policy_hash, report_hash, address_url |
| `data/run_report.json` | written each cycle | wallet_address, policy_hash, run_id, nav, drawdown, events |

The judge-facing proof artifact is defined in
[`crates/bnb-agent/src/proof.rs`](../crates/bnb-agent/src/proof.rs) (`AgentProof`)
and the ERC-8004 / ERC-8183 registry mirrors live alongside it
(`erc8004.rs`, `erc8183.rs`).

## The commitments and how they are derived

Every commitment is a SHA-256 digest in lowercase hex. The verifier reproduces
each rule exactly.

### 1. `policy_hash`

```
policy_hash = sha256( raw bytes of the policy file )
```

Computed by the agent as `bnb_agent::sha256_hex_str(policy_raw)` in
`crates/agent-runtime/src/runtime.rs`, where `policy_raw` is the exact file
content of the active risk policy (`configs/risk_policy.paper.json` in paper mode,
`configs/risk_policy.production.json` in production).

**Independent check:** recompute `sha256` of the policy file and compare. The
verifier confirms that the `policy_hash` in the live `data/run_report.json` equals
the SHA-256 of `configs/risk_policy.paper.json` — so the published policy
commitment is reproducible byte-for-byte.

### 2. `report_hash`

```
report_hash = sha256( compact JSON of the "core" object )
core = { run_id, cycles, final_nav_usd, total_drawdown_pct, events }
```

The agent builds `core` with `serde_json::json!{...}` and hashes
`core.to_string()` (compact, insertion-ordered JSON). The verifier rebuilds the
same object in the same field order, serializes with no whitespace
(`separators=(",", ":")`), and hashes it.

### 3. `agent_id`

```
agent_id = sha256( name + 0x00 + wallet )
```

Defined in [`crates/bnb-agent/src/identity.rs`](../crates/bnb-agent/src/identity.rs).
The name and wallet are joined with a single NUL byte to prevent boundary
collisions. The verifier reproduces this preimage and digest.

### 4. Explorer URLs and the competition contract

- `address_url` must equal `https://bscscan.com/address/<wallet>`
  (`BSCSCAN_BASE_URL` in `crates/bnb-agent/src/proof.rs`).
- `registration_tx`, when anchored, must be a `0x` + 64-hex hash, and its URL
  must equal `https://bscscan.com/tx/<tx>`.
- The competition contract `0x212c61b9b72c95d95bf29cf032f5e5635629aed5` must be a
  well-formed EVM address, and the published explorer URL
  `https://bsctrace.com/address/0x212c61b9b72c95d95bf29cf032f5e5635629aed5` must
  embed it (mirrored from `apps/guardrail-api/src/compete.rs`).

## How to verify (offline)

```bash
# One command — verifies the live run report if present, else the bundled fixture.
./scripts/verify_proof.sh

# Or call the verifier directly on any proof document.
python3 clients/proof-verifier/verify.py data/run_report.json
python3 clients/proof-verifier/verify.py clients/proof-verifier/sample_proof.json

# Pin the policy file used to recompute policy_hash.
python3 clients/proof-verifier/verify.py clients/proof-verifier/sample_proof.json \
  --policy-file configs/risk_policy.paper.json
```

Sample output (bundled fixture, full `/proof` shape):

```
 [PASS] wallet_address      0x-prefixed hex address ...
 [PASS] policy_hash         recomputed sha256 of configs/risk_policy.paper.json matches claimed ...
 [PASS] report_hash         recomputed sha256 over {run_id, cycles, final_nav_usd, total_drawdown_pct, events} matches ...
 [PASS] agent_id            recomputed sha256(name\x00wallet) matches claimed ...
 [PASS] address_url         BscScan address URL well-formed ...
 [PASS] registration_tx     valid tx hash format ...
 [PASS] competition_contract_format        valid EVM address ...
 [PASS] competition_contract_explorer_url  explorer URL embeds the contract ...
 RESULT: PASS  (8 passed, 0 failed, 0 skipped)
```

A bare `data/run_report.json` omits `report_hash`/`agent_id`/`address_url`; those
checks are reported as **SKIP**, while `policy_hash` and the competition-contract
checks still PASS. Use `--strict` to require every commitment to be present.

## Why this is trustworthy

- **No shared code.** The verifier is ~400 lines of dependency-free Python that
  re-implements the hashing rules independently of the Rust agent. Agreement
  between the two is meaningful precisely because they share nothing.
- **Offline and reproducible.** No network, no chain, no secrets. Anyone with the
  repo and `python3` gets the same result.
- **Tamper-evident.** Changing any committed field (policy, report core, wallet,
  URL) changes the recomputed digest and the verifier reports `FAIL` with a
  non-zero exit code — suitable for a CI gate.

## Exit codes

| Code | Meaning |
|------|---------|
| `0`  | All applicable checks passed. |
| `1`  | A check failed (or `--strict` with a skipped check). |
| `2`  | Usage error (missing or invalid proof file). |

## Related

- [`clients/proof-verifier/README.md`](../clients/proof-verifier/README.md) — tool reference.
- [`docs/SELF_CUSTODY.md`](./SELF_CUSTODY.md) — how keys and signing stay with the user.
- [`docs/TWAK_SELF_CUSTODY_DEMO.md`](./TWAK_SELF_CUSTODY_DEMO.md) — the self-custody walkthrough.
