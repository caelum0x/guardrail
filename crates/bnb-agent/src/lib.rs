//! BNB AI-Agent identity, registration, and proof artifacts.
//!
//! This crate produces deterministic, hashable/signable payloads and proof links
//! for BNB-chain AI agents. It performs NO real network or web3 calls: every
//! artifact is derived purely from its inputs so the same inputs always yield the
//! same identifiers, hashes, and explorer URLs.

pub mod erc8004;
pub mod erc8183;
pub mod error;
pub mod identity;
pub mod metadata;
pub mod proof;
pub mod registration;
pub mod report_hash;

pub use erc8004::Erc8004Record;
pub use erc8183::Erc8183Record;
pub use error::AgentError;
pub use identity::AgentIdentity;
pub use metadata::AgentMetadata;
pub use proof::AgentProof;
pub use registration::{
    build_registration_receipt, build_registration_request, RegistrationReceipt,
    RegistrationRequest,
};
pub use report_hash::{sha256_hex, sha256_hex_str};
