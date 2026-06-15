import { API_URL, getJsonOrNull } from "../../lib/api";
import type { ReadinessResponse } from "../../lib/types";

function statusLabel(readiness: ReadinessResponse | null): string {
  if (!readiness) {
    return "API offline";
  }
  return readiness.status === "ready" ? "Ready" : "Blocking";
}

export default async function ReadinessPage() {
  const readiness = await getJsonOrNull<ReadinessResponse>("/readiness");
  const checks = readiness?.checks ?? [];
  const artifacts = Object.entries(readiness?.artifacts ?? {});

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${readiness?.status === "ready" ? "clear" : "critical"}`}>
        <div>
          <p className="eyebrow">Submission readiness</p>
          <h2>{statusLabel(readiness)}</h2>
        </div>
        <div className="metricGrid">
          <div>
            <span>Checks</span>
            <strong>{checks.length}</strong>
          </div>
          <div>
            <span>Blocking</span>
            <strong>{readiness?.blocking ?? checks.length}</strong>
          </div>
          <div>
            <span>Status</span>
            <strong>{statusLabel(readiness)}</strong>
          </div>
        </div>
      </section>

      <section className="panel wide">
        <h2>Checks</h2>
        <div className="alertLedger">
          {checks.map((check) => (
            <div className={`alertRow ${check.status === "pass" ? "clear" : "critical"}`} key={check.id}>
              <strong>{check.label}</strong>
              <span>{check.detail}</span>
              <em>{check.status === "pass" ? "Pass" : "Blocking"}</em>
            </div>
          ))}
        </div>
      </section>

      <section className="panel wide">
        <h2>Artifacts</h2>
        <div className="metricGrid">
          {artifacts.map(([name, path]) => (
            <div key={name}>
              <span>{name.replaceAll("_", " ")}</span>
              <strong>
                <a className="link" href={`${API_URL}${path}`}>
                  {path}
                </a>
              </strong>
            </div>
          ))}
        </div>
      </section>
    </main>
  );
}
