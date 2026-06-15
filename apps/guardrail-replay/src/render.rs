//! Human-readable rendering of events and stats.

use event_store::{AgentEvent, StoredEvent};

use crate::events::event_name;
use crate::stats::{funnel, span, type_counts};

/// Compact one-line JSON for inline display.
fn compact_json(v: &serde_json::Value) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "{}".into())
}

/// Print a chronological decision journal.
pub fn journal(events: &[StoredEvent]) {
    if events.is_empty() {
        println!("(no events recorded)");
        return;
    }
    for e in events {
        println!(
            "{}  {:<28}  {}",
            e.timestamp,
            event_name(&e.event_type),
            compact_json(&e.payload_json)
        );
    }
    println!("\n{} events", events.len());
}

/// Print confirmed on-chain swaps (and any anchored competition tx).
pub fn trades(events: &[StoredEvent]) {
    let confirmed: Vec<&StoredEvent> = events
        .iter()
        .filter(|e| matches!(e.event_type, AgentEvent::TxConfirmed))
        .collect();
    if confirmed.is_empty() {
        println!("(no confirmed transactions)");
        return;
    }
    println!("{:<26}  {:<18}  detail", "timestamp", "tx_hash");
    for e in &confirmed {
        let tx = e
            .payload_json
            .get("tx_hash")
            .and_then(|v| v.as_str())
            .or_else(|| e.payload_json.get("competition_tx").and_then(|v| v.as_str()))
            .unwrap_or("-");
        let short = if tx.len() > 16 { &tx[..16] } else { tx };
        println!("{:<26}  {:<18}  {}", e.timestamp, short, compact_json(&e.payload_json));
    }
    println!("\n{} confirmed transactions", confirmed.len());
}

/// Print risk rejections and clips, surfacing the reasons the engine recorded.
pub fn risk(events: &[StoredEvent]) {
    let gated: Vec<&StoredEvent> = events
        .iter()
        .filter(|e| matches!(e.event_type, AgentEvent::RiskRejected | AgentEvent::RiskClipped))
        .collect();
    if gated.is_empty() {
        println!("(no risk rejections or clips recorded)");
        return;
    }
    println!("{:<26}  {:<12}  reasons", "timestamp", "decision");
    for e in &gated {
        let decision = event_name(&e.event_type);
        let reasons = e
            .payload_json
            .get("reasons")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_else(|| compact_json(&e.payload_json));
        println!("{:<26}  {:<12}  {}", e.timestamp, decision, reasons);
    }
    println!("\n{} risk events", gated.len());
}

/// Print event-type counts in name order.
pub fn summary(events: &[StoredEvent]) {
    for (name, n) in type_counts(events) {
        println!("{n:>5}  {name}");
    }
    println!("\n{} total events", events.len());
}

/// Print the strategy → risk → execution funnel and derived rates.
pub fn stats(events: &[StoredEvent]) {
    let f = funnel(events);
    let s = span(events);
    println!("Guardrail Replay — lifecycle stats\n");
    println!(
        "span: {} events across {} run(s)",
        s.total_events, s.runs
    );
    if let (Some(first), Some(last)) = (&s.first_timestamp, &s.last_timestamp) {
        println!("from:  {first}\nto:    {last}\n");
    } else {
        println!();
    }
    println!("  proposed              {:>6}", f.proposed);
    println!("  risk approved         {:>6}", f.approved);
    println!("  risk rejected         {:>6}", f.rejected);
    println!("  risk clipped          {:>6}", f.clipped);
    println!("  twak quotes           {:>6}", f.quotes);
    println!("  swaps submitted       {:>6}", f.submitted);
    println!("  tx confirmed          {:>6}", f.confirmed);
    println!("  portfolio reconciled  {:>6}", f.reconciled);
    println!("  daily-trade satisfied {:>6}", f.daily_satisfied);
    println!("  throttle activations  {:>6}", f.throttle_activations);
    println!("  kill switches         {:>6}", f.kill_switches);
    println!();
    match f.approval_rate() {
        Some(r) => println!("  approval rate         {:>5.1}%", r * 100.0),
        None => println!("  approval rate            n/a"),
    }
    match f.fill_rate() {
        Some(r) => println!("  fill rate             {:>5.1}%", r * 100.0),
        None => println!("  fill rate                n/a"),
    }
}

/// List distinct runs with event counts and first/last timestamps.
pub fn runs(events: &[StoredEvent]) {
    use std::collections::BTreeMap;
    // run_id -> (count, first, last)
    let mut by_run: BTreeMap<&str, (usize, String, String)> = BTreeMap::new();
    for e in events {
        let entry = by_run
            .entry(e.run_id.as_str())
            .or_insert((0, e.timestamp.clone(), e.timestamp.clone()));
        entry.0 += 1;
        if e.timestamp < entry.1 {
            entry.1 = e.timestamp.clone();
        }
        if e.timestamp > entry.2 {
            entry.2 = e.timestamp.clone();
        }
    }
    if by_run.is_empty() {
        println!("(no runs recorded)");
        return;
    }
    println!("{:<40}  {:>6}  {:<26}  {:<26}", "run_id", "events", "first", "last");
    for (run, (count, first, last)) in &by_run {
        println!("{run:<40}  {count:>6}  {first:<26}  {last:<26}");
    }
    println!("\n{} run(s)", by_run.len());
}
