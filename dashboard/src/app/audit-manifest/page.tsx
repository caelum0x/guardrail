import { getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface AuditArtifact {
  label: string;
  path: string;
  required: boolean;
  exists: boolean;
  size_bytes: number;
  sha256: string;
}

interface AuditRoute {
  path: string;
  declared: boolean;
}

interface AuditResponse {
  name: string;
  generated_for: string;
  status: string;
  summary: {
    artifacts: number;
    present: number;
    missing_required: number;
    routes: number;
    total_bytes: Numeric;
  };
  artifacts: AuditArtifact[];
  routes: AuditRoute[];
  error?: string;
}

function n(value: Numeric): string {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed.toLocaleString("en-US") : "-";
}

function shortHash(value: string): string {
  return value ? `${value.slice(0, 12)}...${value.slice(-8)}` : "-";
}

function statusClass(status: string): string {
  return status === "ready" ? "clear" : "critical";
}

export default async function AuditManifestPage() {
  const data = await getJsonOrNull<AuditResponse>("/audit-manifest");
  const artifacts = Array.isArray(data?.artifacts) ? data.artifacts : [];
  const routes = Array.isArray(data?.routes) ? data.routes : [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(data?.status ?? "missing")}`}>
        <div>
          <h2>Audit Manifest</h2>
          {data?.error ? (
            <p>Failed to load audit manifest: {data.error}</p>
          ) : data ? (
            <p>{data.name}</p>
          ) : (
            <p>Audit manifest unavailable.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Status</span>
              <strong>{data.status}</strong>
            </div>
            <div>
              <span>Artifacts</span>
              <strong>
                {data.summary.present}/{data.summary.artifacts}
              </strong>
            </div>
            <div>
              <span>Missing Required</span>
              <strong>{data.summary.missing_required}</strong>
            </div>
            <div>
              <span>Routes</span>
              <strong>{data.summary.routes}</strong>
            </div>
          </div>
        ) : null}
      </section>

      <section className="panel wide">
        <h2>Artifacts</h2>
        {artifacts.length === 0 ? (
          <p>No artifacts declared.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Artifact</th>
                <th>Status</th>
                <th>Bytes</th>
                <th>SHA-256</th>
              </tr>
            </thead>
            <tbody>
              {artifacts.map((artifact) => (
                <tr key={artifact.path}>
                  <td>
                    <strong>{artifact.label}</strong>
                    <br />
                    <span className="mono">{artifact.path}</span>
                  </td>
                  <td>{artifact.exists ? "present" : artifact.required ? "missing" : "optional"}</td>
                  <td>{n(artifact.size_bytes)}</td>
                  <td className="mono">{shortHash(artifact.sha256)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section className="panel wide">
        <h2>Routes</h2>
        <div className="metricGrid">
          {routes.map((route) => (
            <div key={route.path}>
              <span>{route.path}</span>
              <strong>{route.declared ? "declared" : "missing"}</strong>
            </div>
          ))}
        </div>
      </section>
    </main>
  );
}
