import { getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface Spender {
  name: string;
  address: string;
  address_valid: boolean;
  allowance_usd: Numeric;
  status: string;
}

interface WalletControlsResponse {
  status: string;
  wallet: {
    address: string;
    valid: boolean;
  };
  controls: {
    expected_chain_id: number;
    approval_mode: string;
    max_allowance_usd: Numeric;
    require_quote_before_swap: boolean;
    require_twak_execution: boolean;
    approvals_required: boolean;
  };
  summary: {
    spenders: number;
    violations: number;
  };
  spenders: Spender[];
  error?: string;
}

function n(value: Numeric, digits = 2): string {
  if (value === null || value === undefined) return "-";
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed.toFixed(digits) : String(value);
}

function usd(value: Numeric): string {
  return `$${n(value)}`;
}

function label(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function statusClass(status: string): string {
  return status === "blocking" || status === "violation" ? "critical" : "clear";
}

export default async function WalletControlsPage() {
  const data = await getJsonOrNull<WalletControlsResponse>("/wallet-controls");
  const spenders = Array.isArray(data?.spenders) ? data.spenders : [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(data?.status ?? "blocking")}`}>
        <div>
          <h2>Wallet Controls</h2>
          {data?.error ? (
            <p>Failed to load wallet controls: {data.error}</p>
          ) : !data ? (
            <p>Wallet controls unavailable.</p>
          ) : (
            <p>Self-custody wallet validity and configured spender allowance caps.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Status</span>
              <strong>{label(data.status)}</strong>
            </div>
            <div>
              <span>Wallet Valid</span>
              <strong>{data.wallet.valid ? "true" : "false"}</strong>
            </div>
            <div>
              <span>Spenders</span>
              <strong>{data.summary.spenders}</strong>
            </div>
            <div>
              <span>Violations</span>
              <strong>{data.summary.violations}</strong>
            </div>
          </div>
        ) : null}
      </section>

      {data ? (
        <section className="panel wide">
          <h2>Controls</h2>
          <div className="metricGrid">
            <div>
              <span>Wallet</span>
              <strong className="mono">{data.wallet.address}</strong>
            </div>
            <div>
              <span>Approval Mode</span>
              <strong>{label(data.controls.approval_mode)}</strong>
            </div>
            <div>
              <span>Max Allowance</span>
              <strong>{usd(data.controls.max_allowance_usd)}</strong>
            </div>
            <div>
              <span>TWAK Required</span>
              <strong>{data.controls.require_twak_execution ? "true" : "false"}</strong>
            </div>
          </div>
        </section>
      ) : null}

      <section className="panel wide">
        <h2>Spenders</h2>
        {spenders.length === 0 ? (
          <p>No spenders configured.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>Status</th>
                <th>Allowance</th>
                <th>Address</th>
              </tr>
            </thead>
            <tbody>
              {spenders.map((spender) => (
                <tr key={spender.address}>
                  <td>{spender.name}</td>
                  <td>{label(spender.status)}</td>
                  <td>{usd(spender.allowance_usd)}</td>
                  <td className="mono">{spender.address}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
