pub fn default_policy_json() -> String {
    serde_json::to_string_pretty(&risk_engine::RiskPolicy::default())
        .expect("default policy serializes")
}
