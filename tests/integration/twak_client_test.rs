#[test]
fn twak_fixture_exists() {
    assert!(std::path::Path::new("tests/fixtures/twak_quote_sample.json").exists());
}

