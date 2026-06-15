import { API_URL, getJsonOrNull } from "../../lib/api";

interface ScoreSection {
  id: string;
  label: string;
  weight: number;
  status: string;
  passed_facts: number;
  total_facts: number;
  score_pct: number;
  evidence_routes: string[];
  required_facts: string[];
}

interface ScorecardResponse {
  name: string;
  status: string;
  summary: {
    score_pct: number;
    threshold_ready_pct: number;
    earned_weight: number;
    total_weight: number;
    sections: number;
  };
  sections: ScoreSection[];
  error?: string;
}

function statusClass(status: string): string {
  return status === "ready" ? "clear" : "warning";
}

function label(value: string): string {
  return value
    .split("_")
    .join(" ")
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

export default async function ScorecardPage() {
  const data = await getJsonOrNull<ScorecardResponse>("/scorecard");
  const sections = Array.isArray(data?.sections) ? data.sections : [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(data?.status ?? "partial")}`}>
        <div>
          <h2>Judge Scorecard</h2>
          {data?.error ? (
            <p>Failed to load scorecard: {data.error}</p>
          ) : data ? (
            <p>{data.name}</p>
          ) : (
            <p>Scorecard unavailable.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Status</span>
              <strong>{label(data.status)}</strong>
            </div>
            <div>
              <span>Score</span>
              <strong>{data.summary.score_pct}%</strong>
            </div>
            <div>
              <span>Threshold</span>
              <strong>{data.summary.threshold_ready_pct}%</strong>
            </div>
            <div>
              <span>Sections</span>
              <strong>{data.summary.sections}</strong>
            </div>
          </div>
        ) : null}
      </section>

      {sections.map((section) => (
        <section className={`panel wide statusPanel ${statusClass(section.status)}`} key={section.id}>
          <div>
            <h2>{section.label}</h2>
            <p>
              {section.passed_facts}/{section.total_facts} facts · {section.score_pct}% · weight{" "}
              {section.weight}
            </p>
          </div>
          <div className="metricGrid">
            <div>
              <span>Status</span>
              <strong>{label(section.status)}</strong>
            </div>
            <div>
              <span>Required Facts</span>
              <strong>{section.required_facts.join(", ")}</strong>
            </div>
          </div>
          <div className="stack">
            {section.evidence_routes.map((path) => (
              <a className="link mono" href={`${API_URL}${path}`} key={`${section.id}-${path}`}>
                {path}
              </a>
            ))}
          </div>
        </section>
      ))}
    </main>
  );
}
