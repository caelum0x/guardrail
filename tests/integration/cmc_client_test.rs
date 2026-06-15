#[test]
fn cmc_fixture_exists() {
    assert!(std::path::Path::new("tests/fixtures/cmc_quotes_sample.json").exists());
}

