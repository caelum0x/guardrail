import { getJsonOrNull } from "../../lib/api";

interface SdkModule {
  module: string;
  sdk_path: string;
  guardrail_surface: string;
  status: string;
}

interface BnbSdkResponse {
  source_repo: string;
  local_clone: string;
  network: string;
  chain_id: number;
  competition_contract: string;
  competition_contract_bsctrace: string;
  summary: {
    modules: number;
    implemented_or_referenced: number;
    contracts: number;
    local_files: number;
    local_modules_present: number;
  };
  sdk_modules: SdkModule[];
  sdk_contracts: Record<string, string>;
  error?: string;
}

function label(value: string): string {
  return value
    .split("-")
    .join(" ")
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function statusClass(status: string): string {
  return status === "mapped" ? "warning" : "clear";
}

export default async function BnbSdkPage() {
  const data = await getJsonOrNull<BnbSdkResponse>("/bnb-sdk");
  const modules = Array.isArray(data?.sdk_modules) ? data.sdk_modules : [];
  const contracts = data?.sdk_contracts ? Object.entries(data.sdk_contracts) : [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${data ? "clear" : "critical"}`}>
        <div>
          <h2>BNB Agent SDK</h2>
          {data?.error ? (
            <p>Failed to load BNB SDK map: {data.error}</p>
          ) : data ? (
            <p>SDK clone and module map for BNB AI Agent SDK prize evidence.</p>
          ) : (
            <p>BNB SDK map unavailable.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Network</span>
              <strong>{data.network}</strong>
            </div>
            <div>
              <span>Chain</span>
              <strong>{data.chain_id}</strong>
            </div>
            <div>
              <span>Modules</span>
              <strong>{data.summary.implemented_or_referenced}/{data.summary.modules}</strong>
            </div>
            <div>
              <span>Contracts</span>
              <strong>{data.summary.contracts}</strong>
            </div>
            <div>
              <span>SDK Files</span>
              <strong>{data.summary.local_files}</strong>
            </div>
            <div>
              <span>SDK Modules</span>
              <strong>{data.summary.local_modules_present}/9</strong>
            </div>
          </div>
        ) : null}
      </section>

      {data ? (
        <section className="panel wide">
          <h2>Source</h2>
          <div className="metricGrid">
            <div>
              <span>Repository</span>
              <strong>
                <a className="mono link" href={data.source_repo}>
                  {data.source_repo}
                </a>
              </strong>
            </div>
            <div>
              <span>Local Clone</span>
              <strong className="mono">{data.local_clone}</strong>
            </div>
            <div>
              <span>Competition Contract</span>
              <strong>
                <a className="mono link" href={data.competition_contract_bsctrace}>
                  {data.competition_contract}
                </a>
              </strong>
            </div>
          </div>
        </section>
      ) : null}

      <section className="panel wide">
        <h2>Module Map</h2>
        {modules.length === 0 ? (
          <p>No SDK modules mapped.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>SDK Module</th>
                <th>Status</th>
                <th>Guardrail Surface</th>
              </tr>
            </thead>
            <tbody>
              {modules.map((module) => (
                <tr className={statusClass(module.status)} key={module.sdk_path}>
                  <td>
                    <strong>{module.module}</strong>
                    <br />
                    <span className="mono">{module.sdk_path}</span>
                  </td>
                  <td>{label(module.status)}</td>
                  <td>{module.guardrail_surface}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section className="panel wide">
        <h2>SDK Contracts</h2>
        <div className="metricGrid">
          {contracts.map(([name, address]) => (
            <div key={name}>
              <span>{label(name)}</span>
              <strong className="mono">{address}</strong>
            </div>
          ))}
        </div>
      </section>
    </main>
  );
}
