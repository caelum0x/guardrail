import type { ProofResponse } from "../lib/types";
import { usdString, pctString } from "../lib/format";
import { TxHashLink } from "./TxHashLink";
import { API_URL } from "../lib/api";

export function ProofCard({ proof }: { proof?: ProofResponse | null }) {
  const report = proof?.latest_report;
  const runReport = proof?.run_report;

  return (
    <section className="panel wide">
      <h2>Proof</h2>
      <div className="actions">
        <a className="buttonLink" href={`${API_URL}/report/markdown`}>
          Run report
        </a>
        <a className="buttonLink" href={`${API_URL}/export/submission.md`}>
          Submission export
        </a>
      </div>
      <dl>
        <dt>Agent</dt>
        <dd>{proof?.agent ?? "guardrail-alpha"}</dd>
        <dt>Wallet</dt>
        <dd className="mono">{report?.wallet_address ?? runReport?.wallet_address ?? "Pending"}</dd>
        <dt>Agent id</dt>
        <dd className="mono">{report?.agent_id ?? "Pending"}</dd>
        <dt>Registration tx</dt>
        <dd>{proof?.registration_tx ? <TxHashLink hash={proof.registration_tx} /> : "Pending"}</dd>
        <dt>Run id</dt>
        <dd className="mono">{report?.run_id ?? runReport?.run_id ?? "Pending"}</dd>
        <dt>Final NAV</dt>
        <dd>{usdString(report?.final_nav_usd ?? runReport?.nav_usd)}</dd>
        <dt>Drawdown</dt>
        <dd>{pctString(report?.total_drawdown_pct ?? runReport?.total_drawdown_pct)}</dd>
        <dt>Policy hash</dt>
        <dd className="mono">{report?.policy_hash ?? runReport?.policy_hash ?? "Pending"}</dd>
        <dt>Report hash</dt>
        <dd className="mono">{report?.report_hash ?? "Pending"}</dd>
        <dt>Source event</dt>
        <dd className="mono">{proof?.source_event_id ?? "Pending"}</dd>
      </dl>
    </section>
  );
}
