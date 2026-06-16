"use client";

import { useMemo, useState } from "react";
import type { StoredEvent } from "../../lib/types-expansion";

function compact(payload: Record<string, unknown>): string {
  try {
    const s = JSON.stringify(payload);
    return s.length > 160 ? `${s.slice(0, 160)}…` : s;
  } catch {
    return "{}";
  }
}

export function JournalTable({ events }: { events: StoredEvent[] }) {
  const [type, setType] = useState<string>("all");

  const types = useMemo(() => {
    const set = new Set(events.map((e) => e.event_type));
    return ["all", ...Array.from(set).sort()];
  }, [events]);

  const filtered = type === "all" ? events : events.filter((e) => e.event_type === type);

  return (
    <section className="card">
      <h2>
        Decision journal{" "}
        <span style={{ fontWeight: 400 }}>
          ({filtered.length} of {events.length})
        </span>
      </h2>
      <label>
        Filter by event type:{" "}
        <select value={type} onChange={(e) => setType(e.target.value)}>
          {types.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
      </label>
      <table>
        <thead>
          <tr>
            <th>Timestamp</th>
            <th>Event</th>
            <th>Payload</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((e) => (
            <tr key={e.id}>
              <td>{e.timestamp}</td>
              <td>
                <strong>{e.event_type}</strong>
              </td>
              <td>
                <code>{compact(e.payload_json)}</code>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}
