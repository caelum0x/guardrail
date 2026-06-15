import { getJsonOrNull } from "../../lib/api";

interface LifecycleStep {
  step: number;
  state: string;
  description_hash: string;
  manifest_hash: string;
}

interface JobSimulatorResponse {
  status: string;
  service: {
    id: string;
    label: string;
    price_usd: number;
    endpoint: string;
  };
  job: {
    job_id: number;
    description_hash: string;
  };
  deliverable_hash: string;
  lifecycle: LifecycleStep[];
  error?: string;
}

function short(value: string): string {
  return value ? `${value.slice(0, 12)}...${value.slice(-8)}` : "-";
}

export default async function JobSimulatorPage() {
  const data = await getJsonOrNull<JobSimulatorResponse>("/job-simulator");
  const lifecycle = Array.isArray(data?.lifecycle) ? data.lifecycle : [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${data ? "clear" : "critical"}`}>
        <div>
          <h2>Job Simulator</h2>
          {data?.error ? (
            <p>Failed to load job simulator: {data.error}</p>
          ) : data ? (
            <p>{data.service.label}</p>
          ) : (
            <p>Job simulator unavailable.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Status</span>
              <strong>{data.status}</strong>
            </div>
            <div>
              <span>Job ID</span>
              <strong>{data.job.job_id}</strong>
            </div>
            <div>
              <span>Price</span>
              <strong>{data.service.price_usd.toFixed(2)}</strong>
            </div>
            <div>
              <span>Deliverable Hash</span>
              <strong className="mono">{short(data.deliverable_hash)}</strong>
            </div>
          </div>
        ) : null}
      </section>

      <section className="panel wide">
        <h2>Lifecycle</h2>
        <table>
          <thead>
            <tr>
              <th>Step</th>
              <th>State</th>
              <th>Description Hash</th>
              <th>Manifest Hash</th>
            </tr>
          </thead>
          <tbody>
            {lifecycle.map((step) => (
              <tr key={step.step}>
                <td>{step.step}</td>
                <td>{step.state}</td>
                <td className="mono">{short(step.description_hash)}</td>
                <td className="mono">{short(step.manifest_hash)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </main>
  );
}
