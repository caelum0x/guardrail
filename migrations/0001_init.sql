CREATE TABLE IF NOT EXISTS agent_runs (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    mode TEXT NOT NULL,
    strategy_version TEXT NOT NULL,
    policy_hash TEXT NOT NULL,
    wallet_address TEXT
);

CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload_json TEXT NOT NULL
);

