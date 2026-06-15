import { API_URL, getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface BriefingResponse {
  status: "ready" | "needs_proof" | "blocking" | string;
  title: string;
  claims: string[];
  artifact_paths: string[];
  demo_commands: string[];
  facts: {
    report_path: string;
    report_present: boolean;
    run_id: string | null;
    mode: string | null;
    nav_usd: Numeric;
    wallet_address: string | null;
    policy_hash: string | null;
    events_visible: number;
    confirmed_txs: number;
    risk_decisions: number;
    daily_trade: boolean;
    kill_switch: boolean;
  };
  error?: string;
}

function statusClass(status: string): string {
  if (status === "blocking") {
    return "critical";
  }
  if (status === "needs_proof") {
    return "warning";
  }
  return "clear";
}

function n(value: Numeric): string {
  if (value === null || value === undefined) {
    return "-";
  }
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return String(value);
  }
  return parsed.toFixed(2);
}

function label(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export default async function BriefingPage() {
  const data = await getJsonOrNull<BriefingResponse>("/briefing");
  const claims = Array.isArray(data?.claims) ? data.claims : [];
  const artifacts = Array.isArray(data?.artifact_paths) ? data.artifact_paths : [];
  const commands = Array.isArray(data?.demo_commands) ? data.demo_commands : [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(data?.status ?? "blocking")}`}>
        <div>
          <h2>{data?.title ?? "Submission Briefing"}</h2>
          {data?.error ? (
            <p>Failed to load briefing: {data.error}</p>
          ) : !data ? (
            <p>Briefing unavailable.</p>
          ) : (
            <p>Current report facts, proof links, and judge-facing claims.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Status</span>
              <strong>{label(data.status)}</strong>
            </div>
            <div>
              <span>NAV</span>
              <strong>${n(data.facts.nav_usd)}</strong>
            </div>
            <div>
              <span>Confirmed TXs</span>
              <strong>{data.facts.confirmed_txs}</strong>
            </div>
            <div>
              <span>Daily Trade</span>
              <strong>{data.facts.daily_trade ? "true" : "false"}</strong>
            </div>
          </div>
        ) : null}
      </section>

      <section className="panel">
        <h2>Claims</h2>
        <ul className="plainList">
          {claims.map((claim) => (
            <li key={claim}>{claim}</li>
          ))}
        </ul>
      </section>

      <section className="panel">
        <h2>Artifacts</h2>
        <div className="stack">
          {artifacts.map((path) => (
            <a className="link mono" href={`${API_URL}${path}`} key={path}>
              {path}
            </a>
          ))}
        </div>
      </section>

      <section className="panel wide">
        <h2>Demo Commands</h2>
        <div className="stack">
          {commands.map((command) => (
            <pre key={command}>{command}</pre>
          ))}
        </div>
      </section>

      {data ? (
        <section className="panel wide">
          <h2>Facts</h2>
          <div className="metricGrid">
            <div>
              <span>Run ID</span>
              <strong className="mono">{data.facts.run_id ?? "-"}</strong>
            </div>
            <div>
              <span>Mode</span>
              <strong>{data.facts.mode ?? "-"}</strong>
            </div>
            <div>
              <span>Risk Decisions</span>
              <strong>{data.facts.risk_decisions}</strong>
            </div>
            <div>
              <span>Events</span>
              <strong>{data.facts.events_visible}</strong>
            </div>
          </div>
        </section>
      ) : null}
    </main>
  );
}
