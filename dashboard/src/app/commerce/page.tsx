import { getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface ContractRow {
  name: string;
  address: string;
  bsctrace: string;
}

interface LifecycleRow {
  state: string;
  description: string;
  guardrail_surface: string;
}

interface CommerceResponse {
  name: string;
  status: string;
  network: string;
  chain_id: number;
  agent_role: string;
  service_price_usd: Numeric;
  payment_token_symbol: string;
  agent: {
    wallet_address: string;
    policy_hash: string;
    report_hash: string;
    agent_endpoint: string;
    negotiate_endpoint: string;
    status_endpoint: string;
  };
  summary: {
    contracts: number;
    lifecycle_steps: number;
    deliverables: number;
  };
  contracts: ContractRow[];
  job_lifecycle: LifecycleRow[];
  deliverables: string[];
  error?: string;
}

function n(value: Numeric, digits = 2): string {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed.toFixed(digits) : "-";
}

function label(value: string): string {
  return value
    .split("_")
    .join(" ")
    .split("-")
    .join(" ")
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

function statusClass(status: string): string {
  return status === "ready" ? "clear" : "warning";
}

export default async function CommercePage() {
  const data = await getJsonOrNull<CommerceResponse>("/commerce");
  const contracts = Array.isArray(data?.contracts) ? data.contracts : [];
  const lifecycle = Array.isArray(data?.job_lifecycle) ? data.job_lifecycle : [];
  const deliverables = Array.isArray(data?.deliverables) ? data.deliverables : [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(data?.status ?? "needs_report")}`}>
        <div>
          <h2>ERC-8183 Commerce</h2>
          {data?.error ? (
            <p>Failed to load commerce map: {data.error}</p>
          ) : data ? (
            <p>{data.name}</p>
          ) : (
            <p>Commerce map unavailable.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Status</span>
              <strong>{label(data.status)}</strong>
            </div>
            <div>
              <span>Network</span>
              <strong>{data.network}</strong>
            </div>
            <div>
              <span>Service Price</span>
              <strong>
                {n(data.service_price_usd)} {data.payment_token_symbol}
              </strong>
            </div>
            <div>
              <span>Contracts</span>
              <strong>{data.summary.contracts}</strong>
            </div>
          </div>
        ) : null}
      </section>

      {data ? (
        <section className="panel wide">
          <h2>Provider</h2>
          <div className="metricGrid">
            <div>
              <span>Wallet</span>
              <strong className="mono">{data.agent.wallet_address || "-"}</strong>
            </div>
            <div>
              <span>Policy Hash</span>
              <strong className="mono">{data.agent.policy_hash || "-"}</strong>
            </div>
            <div>
              <span>Report Hash</span>
              <strong className="mono">{data.agent.report_hash || "-"}</strong>
            </div>
            <div>
              <span>Role</span>
              <strong>{label(data.agent_role)}</strong>
            </div>
          </div>
        </section>
      ) : null}

      <section className="panel wide">
        <h2>Job Lifecycle</h2>
        {lifecycle.length === 0 ? (
          <p>No lifecycle steps configured.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>State</th>
                <th>Description</th>
                <th>Guardrail Surface</th>
              </tr>
            </thead>
            <tbody>
              {lifecycle.map((row) => (
                <tr key={row.state}>
                  <td>{label(row.state)}</td>
                  <td>{row.description}</td>
                  <td className="mono">{row.guardrail_surface}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section className="panel wide">
        <h2>Contracts</h2>
        <div className="metricGrid">
          {contracts.map((contract) => (
            <div key={contract.name}>
              <span>{label(contract.name)}</span>
              <strong>
                <a className="mono link" href={contract.bsctrace}>
                  {contract.address}
                </a>
              </strong>
            </div>
          ))}
        </div>
      </section>

      <section className="panel wide">
        <h2>Deliverables</h2>
        <div className="metricGrid">
          {deliverables.map((path) => (
            <div key={path}>
              <span>Route</span>
              <strong className="mono">{path}</strong>
            </div>
          ))}
        </div>
      </section>
    </main>
  );
}
