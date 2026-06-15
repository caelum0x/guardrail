use serde::{Deserialize, Serialize};

/// EVM chain identifier (BSC mainnet = 56).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainId(pub u64);

impl ChainId {
    pub const BSC: ChainId = ChainId(56);

    pub fn is_bsc(&self) -> bool {
        self.0 == 56
    }
}

impl std::fmt::Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A checksummed-or-not EVM address. Stored as a string; validated at boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address(pub String);

impl Address {
    pub fn new(s: impl Into<String>) -> Self {
        Address(s.into())
    }

    /// Loose shape check: `0x` prefix and 42 chars total.
    pub fn looks_valid(&self) -> bool {
        self.0.starts_with("0x") && self.0.len() == 42
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
