pub fn migration_paths() -> [&'static str; 5] {
    [
        "migrations/0001_init.sql",
        "migrations/0002_market_snapshots.sql",
        "migrations/0003_trade_events.sql",
        "migrations/0004_risk_events.sql",
        "migrations/0005_agent_reports.sql",
    ]
}
