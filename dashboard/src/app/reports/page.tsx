import { getJsonOrNull } from "../../lib/api";
import { API_URL } from "../../lib/api";
import { pctString, usdString } from "../../lib/format";
import type { ProofResponse } from "../../lib/types";

export default async function ReportsPage() {
  const proof = await getJsonOrNull<ProofResponse>("/proof");
  const report = proof?.latest_report;
  const runReport = proof?.run_report;
  const positions = runReport?.positions ?? [];

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Reports</h2>
        <div className="actions">
          <a className="buttonLink" href={`${API_URL}/report`}>
            JSON artifact
          </a>
          <a className="buttonLink" href={`${API_URL}/report/markdown`}>
            Markdown report
          </a>
          <a className="buttonLink" href={`${API_URL}/export/submission.md`}>
            Submission markdown
          </a>
        </div>
        <div className="metricGrid">
          <div>
            <span>Run</span>
            <strong className="mono">{report?.run_id ?? runReport?.run_id ?? "Pending"}</strong>
          </div>
          <div>
            <span>Cycles</span>
            <strong>{report?.cycles ?? 0}</strong>
          </div>
          <div>
            <span>Final NAV</span>
            <strong>{usdString(report?.final_nav_usd ?? runReport?.nav_usd)}</strong>
          </div>
          <div>
            <span>Drawdown</span>
            <strong>{pctString(report?.total_drawdown_pct ?? runReport?.total_drawdown_pct)}</strong>
          </div>
          <div>
            <span>Regime</span>
            <strong>{runReport?.regime ?? "Pending"}</strong>
          </div>
          <div>
            <span>Events</span>
            <strong>{report?.events ?? runReport?.events ?? 0}</strong>
          </div>
        </div>
      </section>
      <section className="panel wide">
        <h2>Positions</h2>
        <table>
          <thead>
            <tr>
              <th>Symbol</th>
              <th>Weight</th>
              <th>Value</th>
            </tr>
          </thead>
          <tbody>
            {positions.length === 0 ? (
              <tr>
                <td colSpan={3}>No open risk positions in the latest report.</td>
              </tr>
            ) : (
              positions.map((position) => (
                <tr key={position.symbol}>
                  <td>{position.symbol}</td>
                  <td>{pctString(position.weight_pct)}</td>
                  <td>{usdString(position.value_usd)}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </section>
      <section className="panel wide">
        <h2>Commitments</h2>
        <dl>
          <dt>Wallet</dt>
          <dd className="mono">{report?.wallet_address ?? runReport?.wallet_address ?? "Pending"}</dd>
          <dt>Policy hash</dt>
          <dd className="mono">{report?.policy_hash ?? runReport?.policy_hash ?? "Pending"}</dd>
          <dt>Report hash</dt>
          <dd className="mono">{report?.report_hash ?? "Pending"}</dd>
          <dt>Agent id</dt>
          <dd className="mono">{report?.agent_id ?? "Pending"}</dd>
        </dl>
      </section>
    </main>
  );
}
