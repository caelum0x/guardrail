import { API_URL, getJsonOrNull } from "../../lib/api";

interface Prize {
  id: string;
  label: string;
  claim: string;
  evidence_paths: string[];
  passed_facts: number;
  total_facts: number;
  status: "ready" | "partial" | string;
}

interface PrizesResponse {
  summary: {
    categories: number;
    ready: number;
    partial: number;
  };
  prizes: Prize[];
  error?: string;
}

function label(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function statusClass(status: string): string {
  return status === "ready" ? "clear" : "warning";
}

export default async function PrizesPage() {
  const data = await getJsonOrNull<PrizesResponse>("/prizes");
  const prizes = Array.isArray(data?.prizes) ? data.prizes : [];

  return (
    <main className="grid">
      <section className="panel wide statusPanel clear">
        <div>
          <h2>Prize Map</h2>
          {data?.error ? (
            <p>Failed to load prize map: {data.error}</p>
          ) : !data ? (
            <p>Prize map unavailable.</p>
          ) : (
            <p>Configured prize claims linked to current evidence surfaces.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Categories</span>
              <strong>{data.summary.categories}</strong>
            </div>
            <div>
              <span>Ready</span>
              <strong>{data.summary.ready}</strong>
            </div>
            <div>
              <span>Partial</span>
              <strong>{data.summary.partial}</strong>
            </div>
          </div>
        ) : null}
      </section>

      {prizes.map((prize) => (
        <section className={`panel wide statusPanel ${statusClass(prize.status)}`} key={prize.id}>
          <div>
            <h2>{prize.label}</h2>
            <p>{prize.claim}</p>
          </div>
          <div className="metricGrid">
            <div>
              <span>Status</span>
              <strong>{label(prize.status)}</strong>
            </div>
            <div>
              <span>Facts</span>
              <strong>
                {prize.passed_facts}/{prize.total_facts}
              </strong>
            </div>
          </div>
          <div className="stack">
            {prize.evidence_paths.map((path) => (
              <a className="link mono" href={`${API_URL}${path}`} key={`${prize.id}-${path}`}>
                {path}
              </a>
            ))}
          </div>
        </section>
      ))}
    </main>
  );
}
