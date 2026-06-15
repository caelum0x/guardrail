import { getJsonOrNull } from "../../lib/api";

interface SigningResource {
  label: string;
  resource: string;
  scheme: string;
  network: string;
  amount_base_units: string;
  pay_to: string;
}

interface SigningPolicyResponse {
  name: string;
  status: string;
  mode: string;
  chain_id: number;
  headers: {
    payment: string;
    accepts: string;
  };
  budget: {
    payment_token: string;
    max_per_call_base_units: string;
    session_budget_base_units: string;
    validity_window_seconds: number;
    max_future_validity_seconds: number;
  };
  summary: {
    allowed_types: number;
    denied_types: number;
    resources: number;
    sample_signed: boolean;
  };
  primary_type_allowlist: string[];
  primary_type_denylist: string[];
  resources: SigningResource[];
  sample_payment: {
    resource: string;
    payer: string;
    authorization_hash: string;
    signature: string;
    header_preview: string;
  };
  error?: string;
}

function label(value: string): string {
  return value
    .split("_")
    .join(" ")
    .split("-")
    .join(" ")
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

function short(value: string): string {
  return value && value.length > 28 ? `${value.slice(0, 14)}...${value.slice(-10)}` : value || "-";
}

export default async function SigningPolicyPage() {
  const data = await getJsonOrNull<SigningPolicyResponse>("/signing-policy");
  const resources = Array.isArray(data?.resources) ? data.resources : [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${data ? "clear" : "critical"}`}>
        <div>
          <h2>x402 Signing Policy</h2>
          {data?.error ? (
            <p>Failed to load signing policy: {data.error}</p>
          ) : data ? (
            <p>{data.name}</p>
          ) : (
            <p>Signing policy unavailable.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Mode</span>
              <strong>{label(data.mode)}</strong>
            </div>
            <div>
              <span>Allowed Types</span>
              <strong>{data.summary.allowed_types}</strong>
            </div>
            <div>
              <span>Denied Types</span>
              <strong>{data.summary.denied_types}</strong>
            </div>
            <div>
              <span>Sample Signed</span>
              <strong>{data.summary.sample_signed ? "true" : "false"}</strong>
            </div>
          </div>
        ) : null}
      </section>

      {data ? (
        <section className="panel wide">
          <h2>Budget</h2>
          <div className="metricGrid">
            <div>
              <span>Payment Token</span>
              <strong className="mono">{data.budget.payment_token}</strong>
            </div>
            <div>
              <span>Max Per Call</span>
              <strong>{data.budget.max_per_call_base_units}</strong>
            </div>
            <div>
              <span>Session Budget</span>
              <strong>{data.budget.session_budget_base_units}</strong>
            </div>
            <div>
              <span>Validity Window</span>
              <strong>{data.budget.validity_window_seconds}s</strong>
            </div>
          </div>
        </section>
      ) : null}

      {data ? (
        <section className="panel wide">
          <h2>EIP-712 Types</h2>
          <div className="metricGrid">
            <div>
              <span>Allowlist</span>
              <strong>{data.primary_type_allowlist.join(", ")}</strong>
            </div>
            <div>
              <span>Denylist</span>
              <strong>{data.primary_type_denylist.join(", ")}</strong>
            </div>
            <div>
              <span>Payment Header</span>
              <strong>{data.headers.payment}</strong>
            </div>
            <div>
              <span>Accepts Header</span>
              <strong>{data.headers.accepts}</strong>
            </div>
          </div>
        </section>
      ) : null}

      <section className="panel wide">
        <h2>Resources</h2>
        {resources.length === 0 ? (
          <p>No signing resources configured.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Resource</th>
                <th>Amount</th>
                <th>Network</th>
                <th>Pay To</th>
              </tr>
            </thead>
            <tbody>
              {resources.map((resource) => (
                <tr key={resource.resource}>
                  <td>
                    <strong>{resource.label}</strong>
                    <br />
                    <span className="mono">{resource.resource}</span>
                  </td>
                  <td>{resource.amount_base_units}</td>
                  <td>{resource.network}</td>
                  <td className="mono">{resource.pay_to}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      {data ? (
        <section className="panel wide">
          <h2>Sample Authorization</h2>
          <div className="metricGrid">
            <div>
              <span>Resource</span>
              <strong>{data.sample_payment.resource}</strong>
            </div>
            <div>
              <span>Payer</span>
              <strong className="mono">{data.sample_payment.payer}</strong>
            </div>
            <div>
              <span>Authorization Hash</span>
              <strong className="mono">{short(data.sample_payment.authorization_hash)}</strong>
            </div>
            <div>
              <span>Signature</span>
              <strong className="mono">{short(data.sample_payment.signature)}</strong>
            </div>
          </div>
        </section>
      ) : null}
    </main>
  );
}
