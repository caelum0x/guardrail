# BNB Agent Identity

The `bnb-agent` crate owns identity, off-chain metadata, registry records, and
proof hashes, keeping all of that separate from strategy and execution logic.
Everything here is deterministic and computed off-chain — no network or chain
access — so the same inputs always yield the same identifiers, hashes, and
explorer URLs.

## `AgentIdentity`

Binds a human-readable `name` to a BNB-chain `wallet_address` with an optional
`metadata_url`. The derived `agent_id()` is the lowercase-hex SHA-256 of
`name` + `\0` + `wallet_address` (the NUL separator prevents boundary-ambiguity
collisions). The metadata URL does not affect the id. The runtime computes the
agent id at startup from the agent name and the TWAK wallet address
(`agent-runtime/src/runtime.rs`).

## `AgentMetadata`

The off-chain JSON document an agent publishes (typically at `metadata_url`).
It commits to the agent's behavior by embedding `strategy_hash` and
`policy_hash` (both SHA-256 hex), alongside `name`, `description`, and `version`.
Serializes to/from JSON via `to_json` / `to_json_pretty` / `from_json`.

## Registry records (ERC-8004 / ERC-8183)

`Erc8004Record` and `Erc8183Record` are typed, off-chain mirrors of the
on-chain registry record shapes for the two AI-Agent identity standards. They
are **not** chain transactions — they enumerate exactly the fields that would be
written to the registry so they can be hashed, signed, or shown to a judge
deterministically. Both are built from an `AgentIdentity` + `AgentMetadata` via
`build(...)`.

- `Erc8004Record` (`schema = "erc8004:1"`): `schema`, `agent_id`, `owner`,
  `metadata_uri`, `strategy_hash`, `policy_hash`, `version`.
- `Erc8183Record` (`schema = "erc8183:1"`): `schema`, `agent_id`, `agent_name`,
  `controller`, `metadata_uri`, `policy_hash`, `version`.

## Registration artifacts

`registration.rs` builds the payload that *would* be submitted to a registry.
`build_registration_request(identity, metadata)` assembles a
`RegistrationRequest`; its `registration_id()` is the SHA-256 of the request's
canonical JSON (field-ordered, stable) — a deterministic id with no chain call.
`build_registration_receipt(request, tx)` records the outcome, optionally
carrying a transaction hash anchored out-of-band (e.g. the TWAK competition
registration tx). The on-chain registration itself runs through TWAK — see
[TWAK_INTEGRATION.md](TWAK_INTEGRATION.md).

## Hashing and proof

`report_hash` provides `sha256_hex(bytes)` and `sha256_hex_str(&str)` producing
deterministic lowercase-hex digests for report artifacts and the strategy/policy
commitments.

`AgentProof` (`proof.rs`) is the judge-facing bundle: `agent_id`,
`wallet_address`, optional `registration_tx`, `policy_hash`, and `report_hash`.
It formats BscScan explorer links (`BSCSCAN_BASE_URL = https://bscscan.com`):

- `address_url()` → `https://bscscan.com/address/{wallet_address}`
- `tx_url()` → `https://bscscan.com/tx/{registration_tx}` (when a tx is present)

These are pure string formatting — no network access.

## End-to-end proof flow in the runtime

`agent-runtime/src/runtime.rs` ties the commitments to the running agent:

1. At startup it hashes the loaded risk policy file (`policy_hash =
   sha256_hex_str(policy_raw)`) and derives `agent_id` from the name + wallet.
2. The `AgentStarted` event records `agent_id`, `wallet`, and `policy_hash`.
3. At the end of the run it hashes the run summary (`report_hash`), builds an
   `AgentProof`, and emits the BscScan `address_url` / `registration_tx_url` in
   the `AgentReportPublished` event and the on-disk run report. This lets a judge
   verify the agent's identity, its committed policy, and its published report
   from chain links alone.

## "Best Use of BNB AI Agent SDK" map

| Criterion | Where it lives |
|---|---|
| **Standards compliance** | `Erc8004Record` (`erc8004:1`) and `Erc8183Record` (`erc8183:1`) mirror the BNB AI-Agent identity registry shapes. |
| **Verifiable identity** | `AgentIdentity::agent_id()` deterministic SHA-256; `RegistrationRequest::registration_id()` deterministic over canonical JSON. |
| **Policy/report commitments** | `policy_hash` and `report_hash` bind the running agent to its exact policy and published report. |
| **On-chain proof** | `AgentProof` + BscScan `address_url`/`tx_url`; competition registration tx anchored via TWAK (contract `0x212c61b9b72c95d95bf29cf032f5e5635629aed5`). |
| **Separation of concerns** | Identity/proof isolated in `bnb-agent`; deterministic and chain-free, so artifacts are reproducible and testable. |
