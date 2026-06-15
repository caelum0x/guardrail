//! Tests for identifier helpers.

use common::ids::{new_id, new_run_id};

#[test]
fn new_id_is_non_empty() {
    assert!(!new_id().is_empty());
}

#[test]
fn new_id_is_unique_across_calls() {
    assert_ne!(new_id(), new_id());
}

#[test]
fn new_run_id_has_run_prefix() {
    let id = new_run_id();
    assert!(id.starts_with("run_"), "expected run_ prefix, got {id}");
    // Something follows the prefix.
    assert!(id.len() > "run_".len());
}

#[test]
fn new_run_id_is_unique_across_calls() {
    assert_ne!(new_run_id(), new_run_id());
}
