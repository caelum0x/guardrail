import { getJsonOrNull } from "../../lib/api";
import type { EventsResponse, StoredEvent } from "../../lib/types-expansion";
import { JournalTable } from "./JournalTable";

export default async function JournalProPage() {
  const data = await getJsonOrNull<EventsResponse>("/events");
  const events: StoredEvent[] = data?.events ?? [];

  return (
    <main className="grid">
      <section className="card">
        <h1>Decision Journal (Pro)</h1>
        <p>
          The agent&apos;s recent append-only events with a client-side type
          filter &mdash; the full audit trail of why it traded, what it quoted,
          what risk decided, and what settled.
        </p>
        {events.length === 0 ? <p>No events yet (run the agent first).</p> : null}
      </section>
      {events.length > 0 ? <JournalTable events={events} /> : null}
    </main>
  );
}
