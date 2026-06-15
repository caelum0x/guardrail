use event_store::{AgentEvent, SqliteEventRepository};
use serde_json::json;

#[test]
fn sqlite_repository_appends_and_reads_recent_events() {
    let repo = SqliteEventRepository::new_in_memory().expect("in-memory sqlite opens");

    repo.append(
        "run-1",
        AgentEvent::AgentStarted,
        json!({ "mode": "paper" }),
    )
    .expect("append agent start");
    repo.append(
        "run-1",
        AgentEvent::RiskApproved,
        json!({ "order_id": "order-1" }),
    )
    .expect("append risk approval");

    let recent = repo.recent(10).expect("read recent events");
    assert_eq!(recent.len(), 2);
    assert!(matches!(recent[0].event_type, AgentEvent::RiskApproved));
    assert_eq!(recent[0].payload_json["order_id"], "order-1");
    assert!(matches!(recent[1].event_type, AgentEvent::AgentStarted));
}

#[test]
fn sqlite_repository_honors_recent_limit() {
    let repo = SqliteEventRepository::new_in_memory().expect("in-memory sqlite opens");

    for i in 0..3 {
        repo.append("run-1", AgentEvent::AssetScored, json!({ "rank": i }))
            .expect("append event");
    }

    let recent = repo.recent(2).expect("read recent events");
    assert_eq!(recent.len(), 2);
}
