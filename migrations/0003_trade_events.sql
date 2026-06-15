CREATE TABLE IF NOT EXISTS trade_events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    from_symbol TEXT NOT NULL,
    to_symbol TEXT NOT NULL,
    amount_usd REAL NOT NULL,
    status TEXT NOT NULL,
    quote_json TEXT,
    risk_decision_json TEXT,
    tx_hash TEXT,
    reason TEXT
);

