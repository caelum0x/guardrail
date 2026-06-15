import { getJsonOrNull } from "../../lib/api";
import { compactDate, labelEvent } from "../../lib/format";
import type { EventsResponse, StoredEvent } from "../../lib/types";

function countByType(events: StoredEvent[]): Array<[string, number]> {
  const counts = new Map<string, number>();
  for (const event of events) {
    counts.set(event.event_type, (counts.get(event.event_type) ?? 0) + 1);
  }
  return Array.from(counts.entries()).sort((a, b) => b[1] - a[1]);
}

function payloadPreview(event: StoredEvent): string {
  return JSON.stringify(event.payload_json);
}

export default async function EventsPage() {
  const data = await getJsonOrNull<EventsResponse>("/events");
  const events = data?.events ?? [];
  const byType = countByType(events);
  const latest = events[0];
  const riskEvents = events.filter((event) => event.event_type.startsWith("risk_")).length;
  const executionEvents = events.filter((event) =>
    ["twak_quote_received", "twak_swap_submitted", "tx_confirmed"].includes(event.event_type),
  ).length;

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Event Log</h2>
        <div className="metricGrid">
          <div>
            <span>Total visible</span>
            <strong>{events.length}</strong>
          </div>
          <div>
            <span>Event types</span>
            <strong>{byType.length}</strong>
          </div>
          <div>
            <span>Risk events</span>
            <strong>{riskEvents}</strong>
          </div>
          <div>
            <span>Execution events</span>
            <strong>{executionEvents}</strong>
          </div>
          <div>
            <span>Latest</span>
            <strong>{latest ? compactDate(latest.timestamp) : "Pending"}</strong>
          </div>
        </div>
      </section>

      <section className="panel">
        <h2>Type Counts</h2>
        <div className="stack">
          {byType.length === 0 ? (
            <p>No events visible.</p>
          ) : (
            byType.map(([eventType, count]) => (
              <div className="eventRow" key={eventType}>
                <span>{labelEvent(eventType)}</span>
                <strong>{count}</strong>
              </div>
            ))
          )}
        </div>
      </section>

      <section className="panel">
        <h2>Latest Event</h2>
        {latest ? (
          <dl>
            <dt>Type</dt>
            <dd>{labelEvent(latest.event_type)}</dd>
            <dt>Run</dt>
            <dd className="mono">{latest.run_id}</dd>
            <dt>Time</dt>
            <dd>{latest.timestamp}</dd>
          </dl>
        ) : (
          <p>No latest event.</p>
        )}
      </section>

      <section className="panel wide">
        <h2>Recent Payloads</h2>
        <table>
          <thead>
            <tr>
              <th>Time</th>
              <th>Event</th>
              <th>Run</th>
              <th>Payload</th>
            </tr>
          </thead>
          <tbody>
            {events.slice(0, 80).map((event) => (
              <tr key={event.id}>
                <td>{compactDate(event.timestamp)}</td>
                <td>{labelEvent(event.event_type)}</td>
                <td className="mono">{event.run_id}</td>
                <td className="mono">{payloadPreview(event)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </main>
  );
}
