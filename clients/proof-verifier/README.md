# Guardrail Proof Verifier

An independent, **stdlib-only** tool that verifies the Guardrail BNB AI-Agent's
on-chain *identity proof* offline — without trusting the agent and without any
network, chain, or third-party dependencies.

The agent publishes a proof (via its `/proof` HTTP route, the
`AgentReportPublished` event, or the on-disk `data/run_report.json`) that commits
to its policy and its run report via SHA-256 hashes, plus a deterministic
`agent_id` and BscScan explorer links. This tool **re-derives every one of those
commitments from first principles** and compares them to the claimed values. A
third party can therefore confirm the agent's identity is genuinely verifiable,
not merely asserted.

## What it checks

| Check | How it is independently verified |
|-------|----------------------------------|
| `policy_hash` | Recompute `sha256(bytes of the policy file)` and compare to the claimed value. Mirrors `crates/agent-runtime/src/runtime.rs` (`sha256_hex_str(policy_raw)`). |
| `report_hash` | Recompute `sha256(compact JSON of {run_id, cycles, final_nav_usd, total_drawdown_pct, events})` and compare. Mirrors the agent's `core` object hashing. |
| `agent_id` | Recompute `sha256(name + 0x00 + wallet)` and compare. Mirrors `crates/bnb-agent/src/identity.rs`. |
| `wallet_address` | Validate the `0x`-prefixed hex EVM address format. |
| `address_url` | Confirm it equals `https://bscscan.com/address/<wallet>`. Mirrors `crates/bnb-agent/src/proof.rs`. |
| `registration_tx` | If present, validate the `0x` + 64-hex tx-hash format and the `https://bscscan.com/tx/<tx>` URL. |
| `competition_contract` | Validate the competition contract address and that the published BscTrace explorer URL embeds it. Mirrors `apps/guardrail-api/src/compete.rs`. |

Checks for commitments that a particular proof shape does not carry (for
example, a bare `run_report.json` omits `report_hash` and `agent_id`) are
reported as **SKIP** rather than failing. Use `--strict` to treat skips as
failures.

### Optional on-chain checks (`--rpc`)

By default the tool is fully offline. Passing `--rpc <BSC_RPC_URL>` (or setting
the `BSC_RPC_URL` environment variable) adds **read-only** BSC JSON-RPC checks,
using only `urllib` from the standard library. These mirror the `chain-verifier`
crate exactly:

| Check | RPC method | What it proves |
|-------|------------|----------------|
| `onchain_chain_id` | `eth_chainId` | The endpoint is BSC mainnet (chain id `56`). |
| `onchain_contract_code` | `eth_getCode` | The competition contract has deployed bytecode on-chain. |
| `onchain_registration_receipt` | `eth_getTransactionReceipt` | The registration tx (when present) was mined with `status=success` and `to` = the competition contract. |

When no RPC is supplied these checks are **SKIP**, so the default run stays
offline and dependency-free. The checks are ABI-free — every claim is one you can
reproduce with `cast`/`curl` against the same RPC.

## Usage

```bash
# Verify the live run report if present, else the bundled offline fixture.
python3 verify.py

# Verify a specific proof document.
python3 verify.py ../../data/run_report.json
python3 verify.py sample_proof.json

# Recompute policy_hash against an explicit policy file.
python3 verify.py sample_proof.json --policy-file ../../configs/risk_policy.paper.json

# Require every commitment to be present (skips become failures).
python3 verify.py sample_proof.json --strict

# Machine-readable output for CI.
python3 verify.py sample_proof.json --json

# Add read-only on-chain verification against the live chain.
python3 verify.py --rpc https://bsc-dataseed.bnbchain.org
# → onchain_contract_code PASS: competition contract 0x212c61… has deployed bytecode (2510 bytes)
```

Or via the repo wrapper, which auto-selects the run report or the fixture:

```bash
./scripts/verify_proof.sh
./scripts/verify_proof.sh path/to/proof.json
```

## Exit codes

| Code | Meaning |
|------|---------|
| `0`  | Every applicable check passed (PASS). |
| `1`  | At least one check failed (FAIL), or `--strict` and a check was skipped. |
| `2`  | Usage error (missing/invalid proof file). |

## Offline fixture

`sample_proof.json` is a self-contained `/proof`-shaped document whose
`policy_hash` is the real SHA-256 of `configs/risk_policy.paper.json`, with a
self-consistent `report_hash`, `agent_id`, `address_url`, and `registration_tx`.
It lets the verifier demonstrate a full PASS even with no run report present.

## Why this matters

The verifier deliberately shares **no code** with the Rust agent — it is a
clean-room re-implementation of the hashing rules in ~400 lines of dependency-free
Python. If the verifier and the agent agree, the agent's published identity and
report commitments are reproducible by anyone, which is the property the BNB
prize's "verifiable identity" criterion rewards. See
[`docs/PROOF_VERIFICATION.md`](../../docs/PROOF_VERIFICATION.md) for the full
third-party verification narrative.
