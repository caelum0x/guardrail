//! SHA-256 hashing helpers for report artifacts.
//!
//! These produce deterministic, lowercase hex digests suitable for embedding in
//! signable / hashable proof payloads. No network or chain access is performed.

use sha2::{Digest, Sha256};

/// Computes the SHA-256 digest of raw bytes and returns it as lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Computes the SHA-256 digest of a UTF-8 string and returns it as lowercase hex.
pub fn sha256_hex_str(input: &str) -> String {
    sha256_hex(input.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_matches_known_sha256() {
        // Well-known SHA-256 of the empty input.
        assert_eq!(
            sha256_hex(&[]),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn string_helper_matches_byte_helper() {
        assert_eq!(sha256_hex_str("guardrail"), sha256_hex(b"guardrail"));
    }

    #[test]
    fn hashing_is_deterministic() {
        assert_eq!(sha256_hex_str("report-001"), sha256_hex_str("report-001"));
    }
}
