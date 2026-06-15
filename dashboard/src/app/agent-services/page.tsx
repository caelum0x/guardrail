import { getJsonOrNull } from "../../lib/api";

interface AgentService {
  id: string;
  label: string;
  price_usd: number;
  sla_minutes: number;
  endpoint: string;
  deliverables: string[];
  required_inputs: string[];
  job_description_hash: string;
}

interface AgentServicesResponse {
  name: string;
  provider: string;
  network: string;
  currency: string;
  status: string;
  commerce: {
    payment_token: string;
    commerce_proxy: string;
    router_proxy: string;
    policy: string;
  };
  summary: {
    services: number;
    total_catalog_price_usd: number;
    deliverable_routes: number;
  };
  services: AgentService[];
  error?: string;
}

function short(value: string): string {
  return value.length > 24 ? `${value.slice(0, 12)}...${value.slice(-8)}` : value;
}

function statusClass(status: string): string {
  return status === "listed" ? "clear" : "warning";
}

export default async function AgentServicesPage() {
  const data = await getJsonOrNull<AgentServicesResponse>("/agent-services");
  const services = Array.isArray(data?.services) ? data.services : [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(data?.status ?? "empty")}`}>
        <div>
          <h2>Agent Services</h2>
          {data?.error ? (
            <p>Failed to load agent services: {data.error}</p>
          ) : data ? (
            <p>{data.name}</p>
          ) : (
            <p>Agent services unavailable.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Provider</span>
              <strong>{data.provider}</strong>
            </div>
            <div>
              <span>Network</span>
              <strong>{data.network}</strong>
            </div>
            <div>
              <span>Services</span>
              <strong>{data.summary.services}</strong>
            </div>
            <div>
              <span>Catalog Price</span>
              <strong>
                {data.summary.total_catalog_price_usd.toFixed(2)} {data.currency}
              </strong>
            </div>
          </div>
        ) : null}
      </section>

      {data ? (
        <section className="panel wide">
          <h2>Commerce Contracts</h2>
          <div className="metricGrid">
            <div>
              <span>Payment Token</span>
              <strong className="mono">{data.commerce.payment_token}</strong>
            </div>
            <div>
              <span>Commerce</span>
              <strong className="mono">{data.commerce.commerce_proxy}</strong>
            </div>
            <div>
              <span>Router</span>
              <strong className="mono">{data.commerce.router_proxy}</strong>
            </div>
            <div>
              <span>Policy</span>
              <strong className="mono">{data.commerce.policy}</strong>
            </div>
          </div>
        </section>
      ) : null}

      <section className="panel wide">
        <h2>Services</h2>
        <table>
          <thead>
            <tr>
              <th>Service</th>
              <th>Price</th>
              <th>SLA</th>
              <th>Endpoint</th>
              <th>Hash</th>
            </tr>
          </thead>
          <tbody>
            {services.map((service) => (
              <tr key={service.id}>
                <td>
                  <strong>{service.label}</strong>
                  <br />
                  <span>{service.deliverables.join(", ")}</span>
                </td>
                <td>{service.price_usd.toFixed(2)}</td>
                <td>{service.sla_minutes}m</td>
                <td className="mono">{service.endpoint}</td>
                <td className="mono">{short(service.job_description_hash)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </main>
  );
}
