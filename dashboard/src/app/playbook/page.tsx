import { getJsonOrNull } from "../../lib/api";

interface PlaybookItem {
  id: string;
  status: string;
  label: string;
  description: string;
  commands: string[];
}

interface PlaybookResponse {
  active_id: string;
  active: PlaybookItem;
  playbooks: PlaybookItem[];
  facts: {
    report_path: string;
    report_present: boolean;
    kill_switch: boolean;
    events_visible: number;
    confirmed_txs: number;
    risk_decisions: number;
  };
  error?: string;
}

function statusClass(status: string): string {
  if (status === "critical") {
    return "critical";
  }
  if (status === "warning" || status === "blocking") {
    return "warning";
  }
  return "clear";
}

function label(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export default async function PlaybookPage() {
  const data = await getJsonOrNull<PlaybookResponse>("/playbook");
  const active = data?.active ?? null;
  const commands = Array.isArray(active?.commands) ? active.commands : [];
  const all = Array.isArray(data?.playbooks) ? data.playbooks : [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(active?.status ?? "critical")}`}>
        <div>
          <h2>Operator Playbook</h2>
          {data?.error ? (
            <p>Failed to load playbook: {data.error}</p>
          ) : !data || !active ? (
            <p>Playbook unavailable.</p>
          ) : (
            <p>{active.description}</p>
          )}
        </div>
        {data && active ? (
          <div className="metricGrid">
            <div>
              <span>Active</span>
              <strong>{active.label}</strong>
            </div>
            <div>
              <span>Status</span>
              <strong>{label(active.status)}</strong>
            </div>
            <div>
              <span>Events</span>
              <strong>{data.facts.events_visible}</strong>
            </div>
            <div>
              <span>Confirmed TXs</span>
              <strong>{data.facts.confirmed_txs}</strong>
            </div>
          </div>
        ) : null}
      </section>

      <section className="panel wide">
        <h2>Commands</h2>
        {commands.length === 0 ? (
          <p>No commands available.</p>
        ) : (
          <div className="stack">
            {commands.map((command) => (
              <pre key={command}>{command}</pre>
            ))}
          </div>
        )}
      </section>

      {data ? (
        <section className="panel wide">
          <h2>Facts</h2>
          <div className="metricGrid">
            <div>
              <span>Report</span>
              <strong>{data.facts.report_present ? "present" : "missing"}</strong>
            </div>
            <div>
              <span>Kill Switch</span>
              <strong>{data.facts.kill_switch ? "true" : "false"}</strong>
            </div>
            <div>
              <span>Risk Decisions</span>
              <strong>{data.facts.risk_decisions}</strong>
            </div>
            <div>
              <span>Report Path</span>
              <strong className="mono">{data.facts.report_path}</strong>
            </div>
          </div>
        </section>
      ) : null}

      <section className="panel wide">
        <h2>All Playbooks</h2>
        {all.length === 0 ? (
          <p>No playbooks configured.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>ID</th>
                <th>Status</th>
                <th>Label</th>
                <th>Commands</th>
              </tr>
            </thead>
            <tbody>
              {all.map((item) => (
                <tr key={item.id}>
                  <td>{item.id}</td>
                  <td>{label(item.status)}</td>
                  <td>{item.label}</td>
                  <td>{item.commands.length}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
