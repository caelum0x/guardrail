//! Read-only on-chain verification for the Guardrail BNB AI-Agent.
//!
//! This crate performs **read-only** BSC JSON-RPC calls — `eth_chainId`,
//! `eth_getCode`, and `eth_getTransactionReceipt` — to independently confirm
//! that the agent's claimed registration is anchored on-chain. It never signs,
//! never sends value, and holds no keys.
//!
//! It is the missing half of the proof story: the rest of the pipeline
//! (`bnb-agent`, `clients/proof-verifier`, `/proof/verify`) recomputes hashes
//! and validates *formats* purely offline. This crate answers the questions
//! those checks cannot — is the competition contract actually deployed, are we
//! on BSC mainnet, and was the registration transaction really mined?
//!
//! Everything is **offline-safe**: when `BSC_RPC_URL` is unset the verifier
//! returns a single `Skipped` check rather than erroring, so the offline
//! paper/demo flow stays green. RPC failures degrade to `Fail` checks with the
//! underlying error in the detail — never a panic.

pub mod rpc;
pub mod verifier;

pub use rpc::{parse_hex_u64, BscRpcClient, Receipt, RpcError};
pub use verifier::{rpc_url_from_env, verify_onchain, CheckStatus, OnChainCheck, OnChainReport};
