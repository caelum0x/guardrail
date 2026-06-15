#[test]
fn risk_policy_fixture_exists() {
    assert!(std::path::Path::new("tests/fixtures/risk_policy_sample.json").exists());
}

