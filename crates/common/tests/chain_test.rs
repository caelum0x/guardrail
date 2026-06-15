//! Tests for chain identifiers and addresses.

use common::chain::{Address, ChainId};

#[test]
fn bsc_constant_is_chain_56_and_is_bsc() {
    assert_eq!(ChainId::BSC.0, 56);
    assert!(ChainId::BSC.is_bsc());
}

#[test]
fn non_bsc_chain_is_not_bsc() {
    assert!(!ChainId(1).is_bsc());
}

#[test]
fn chain_id_display() {
    assert_eq!(ChainId::BSC.to_string(), "56");
}

#[test]
fn address_looks_valid_for_42_char_0x_string() {
    // "0x" + 40 hex chars = 42 chars total.
    let addr = Address::new("0x55d398326f99059ff775485246999027b3197955");
    assert_eq!(addr.0.len(), 42);
    assert!(addr.looks_valid());
}

#[test]
fn address_looks_invalid_without_prefix_or_wrong_length() {
    // Wrong length (too short).
    assert!(!Address::new("0x1234").looks_valid());
    // Missing 0x prefix but correct length.
    assert!(!Address::new("55d398326f99059ff775485246999027b31979551x").looks_valid());
    // Empty.
    assert!(!Address::new("").looks_valid());
}

#[test]
fn address_display() {
    let addr = Address::new("0xabc");
    assert_eq!(addr.to_string(), "0xabc");
}
