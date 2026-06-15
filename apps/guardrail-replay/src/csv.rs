//! CSV export of the event log.

use event_store::StoredEvent;

use crate::events::event_name;

/// Escape a field for CSV (quote and double internal quotes).
pub fn csv_escape(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// Render all events as a CSV document with a header row.
pub fn to_csv(events: &[StoredEvent]) -> String {
    let mut out = String::from("id,run_id,timestamp,event_type,payload\n");
    for e in events {
        out.push_str(&format!(
            "{},{},{},{},{}\n",
            e.id,
            e.run_id,
            e.timestamp,
            event_name(&e.event_type),
            csv_escape(&e.payload_json.to_string()),
        ));
    }
    out
}

/// Write the CSV to `path`, or to stdout when `path` is `-`.
pub fn export(events: &[StoredEvent], path: &str) -> anyhow::Result<()> {
    let out = to_csv(events);
    if path == "-" {
        print!("{out}");
    } else {
        std::fs::write(path, out)?;
        println!("wrote {} events to {path}", events.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::stub;
    use event_store::AgentEvent;

    #[test]
    fn escapes_embedded_quotes() {
        assert_eq!(csv_escape(r#"a"b"#), r#""a""b""#);
    }

    #[test]
    fn csv_has_header_and_row_per_event() {
        let events = vec![
            stub("r1", "t1", AgentEvent::AgentStarted),
            stub("r1", "t2", AgentEvent::TxConfirmed),
        ];
        let csv = to_csv(&events);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 rows
        assert!(lines[0].starts_with("id,run_id"));
        assert!(csv.contains("tx_confirmed"));
    }
}
