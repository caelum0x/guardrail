import { getJsonOrNull } from "../../lib/api";

interface AgentEndpoint {
  name: string;
  endpoint: string;
  version?: string;
  capabilities?: string[];
}

interface AgentCardResponse {
  status: string;
  summary: {
    services: number;
    registrations: number;
    supported_trust: number;
  };
  agent_uri: string;
  registration_hash: string;
  card: {
    name: string;
    description: string;
    services: AgentEndpoint[];
    registrations: Array<{ agentId: number; agentRegistry: string }>;
    supportedTrust: string[];
  };
  error?: string;
}

function short(value: string): string {
  return value.length > 32 ? `${value.slice(0, 16)}...${value.slice(-10)}` : value;
}

export default async function AgentCardPage() {
  const data = await getJsonOrNull<AgentCardResponse>("/agent-card");
  const endpoints = Array.isArray(data?.card?.services) ? data.card.services : [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${data ? "clear" : "critical"}`}>
        <div>
          <h2>Agent Card</h2>
          {data?.error ? (
            <p>Failed to load agent card: {data.error}</p>
          ) : data ? (
            <p>{data.card.name}</p>
          ) : (
            <p>Agent card unavailable.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Services</span>
              <strong>{data.summary.services}</strong>
            </div>
            <div>
              <span>Registrations</span>
              <strong>{data.summary.registrations}</strong>
            </div>
            <div>
              <span>Trust Modes</span>
              <strong>{data.summary.supported_trust}</strong>
            </div>
            <div>
              <span>Hash</span>
              <strong className="mono">{short(data.registration_hash)}</strong>
            </div>
          </div>
        ) : null}
      </section>

      <section className="panel wide">
        <h2>Endpoints</h2>
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Endpoint</th>
              <th>Version</th>
              <th>Capabilities</th>
            </tr>
          </thead>
          <tbody>
            {endpoints.map((endpoint) => (
              <tr key={`${endpoint.name}-${endpoint.endpoint}`}>
                <td>{endpoint.name}</td>
                <td className="mono">{endpoint.endpoint}</td>
                <td>{endpoint.version ?? "-"}</td>
                <td>{endpoint.capabilities?.join(", ") ?? "-"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      {data ? (
        <section className="panel wide">
          <h2>Agent URI</h2>
          <p className="mono">{short(data.agent_uri)}</p>
          <p>{data.card.description}</p>
        </section>
      ) : null}
    </main>
  );
}
