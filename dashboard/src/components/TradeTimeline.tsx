import type { StoredEvent } from "../lib/types";
import { compactDate, labelEvent } from "../lib/format";
import { TxHashLink } from "./TxHashLink";

export function TradeTimeline({ events = [] }: { events?: StoredEvent[] }) {
  const visible = events.slice(0, 12);

  return (
    <section className="panel">
      <h2>Trades</h2>
      {visible.length === 0 ? (
        <p>No trades recorded yet.</p>
      ) : (
        <ol className="timeline">
          {visible.map((event) => {
            const hash = typeof event.payload_json.tx_hash === "string" ? event.payload_json.tx_hash : null;
            return (
              <li key={event.id}>
                <div>
                  <strong>{labelEvent(event.event_type)}</strong>
                  <span>{compactDate(event.timestamp)}</span>
                </div>
                {hash ? <TxHashLink hash={hash} /> : <code>{JSON.stringify(event.payload_json)}</code>}
              </li>
            );
          })}
        </ol>
      )}
    </section>
  );
}
