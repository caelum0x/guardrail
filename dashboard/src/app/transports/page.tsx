import { getJsonOrNull } from "../../lib/api";
import type { EventsResponse, StoredEvent } from "../../lib/types-expansion";

function str(payload: Record<string, unknown>, key: string): string | null {
  const v = payload[key];
  return typeof v === "string" ? v : null;
}

function isMock(transport: string | null): boolean {
  return transport === "mock" || transport === null;
}

export default async function TransportsPage() {
  const data = await getJsonOrNull<EventsResponse>("/events");
  const events: StoredEvent[] = data?.events ?? [];
  // /events is newest-first; the most recent agent_started carries the chosen transports.
  const started = events.find((e) => e.event_type === "agent_started");
  const payload = started?.payload_json ?? {};
  const cmc = str(payload, "cmc_transport");
  const twak = str(payload, "twak_transport");
  const mode = str(payload, "mode");

  return (
    <main className="grid">
      <section className="card">
        <h1>Transports</h1>
        <p>
          Which data and execution transports the running agent resolved at
          startup &mdash; so an operator can see live-vs-mock at a glance. In{" "}
          <code>live</code> mode the agent refuses to start on mock transports.
        </p>
        {!started ? (
          <p>No <code>agent_started</code> event yet (run the agent first).</p>
        ) : (
          <table>
            <tbody>
              <tr>
                <td>Mode</td>
                <td>
                  <strong>{mode ?? "unknown"}</strong>
                </td>
              </tr>
              <tr>
                <td>CMC data transport</td>
                <td>
                  {isMock(cmc) ? "🟡" : "🟢"} <strong>{cmc ?? "unknown"}</strong>
                </td>
              </tr>
              <tr>
                <td>TWAK execution transport</td>
                <td>
                  {isMock(twak) ? "🟡" : "🟢"} <strong>{twak ?? "unknown"}</strong>
                </td>
              </tr>
              <tr>
                <td>Run</td>
                <td>
                  <code>{started.run_id}</code>
                </td>
              </tr>
              <tr>
                <td>Started</td>
                <td>{started.timestamp}</td>
              </tr>
            </tbody>
          </table>
        )}
        {mode === "live" && (isMock(cmc) || isMock(twak)) ? (
          <p>
            ⚠️ Live mode with a mock transport should be impossible — the agent
            fails fast on missing credentials. Investigate.
          </p>
        ) : null}
      </section>
    </main>
  );
}
