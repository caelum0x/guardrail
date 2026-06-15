import type { RiskResponse } from "../lib/types";
import { compactDate, labelEvent } from "../lib/format";

export function RiskPanel({ risk }: { risk?: RiskResponse | null }) {
  const events = risk?.events?.slice(0, 8) ?? [];

  return (
    <section className="panel">
      <h2>Risk</h2>
      <dl>
        <dt>Kill switch</dt>
        <dd>{risk?.kill_switch ? "Active" : "Inactive"}</dd>
        <dt>Execution layer</dt>
        <dd>TWAK only</dd>
      </dl>
      <div className="stack">
        {events.length === 0 ? (
          <p>No risk decisions recorded yet.</p>
        ) : (
          events.map((event) => (
            <div className="eventRow" key={event.id}>
              <span>{labelEvent(event.event_type)}</span>
              <strong>{compactDate(event.timestamp)}</strong>
            </div>
          ))
        )}
      </div>
    </section>
  );
}
