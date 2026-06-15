CREATE TABLE IF NOT EXISTS agent_reports (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    report_date TEXT NOT NULL,
    report_markdown TEXT NOT NULL,
    report_hash TEXT NOT NULL,
    published_tx_hash TEXT
);

