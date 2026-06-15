import { getJsonOrNull } from "../../lib/api";
import type { PolicyResponse } from "../../lib/types";
import { Fragment } from "react";

function renderLimit(policy: Record<string, unknown> | undefined, key: string): string {
  const value = policy?.[key];
  return value === undefined || value === null ? "Pending" : String(value);
}

export default async function PolicyPage() {
  const policy = await getJsonOrNull<PolicyResponse>("/policy");
  const production = policy?.production.value;

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Risk Policy</h2>
        <div className="metricGrid">
          <div>
            <span>Total drawdown</span>
            <strong>{renderLimit(production, "max_total_drawdown_pct")}%</strong>
          </div>
          <div>
            <span>Daily drawdown</span>
            <strong>{renderLimit(production, "max_daily_drawdown_pct")}%</strong>
          </div>
          <div>
            <span>Position cap</span>
            <strong>{renderLimit(production, "max_position_pct")}%</strong>
          </div>
          <div>
            <span>Slippage cap</span>
            <strong>{renderLimit(production, "max_slippage_pct")}%</strong>
          </div>
        </div>
      </section>
      <section className="panel">
        <h2>Enforcement</h2>
        <dl>
          {Object.entries(policy?.enforcement ?? {}).map(([key, value]) => (
            <Fragment key={key}>
              <dt>{key.replaceAll("_", " ")}</dt>
              <dd>{String(value)}</dd>
            </Fragment>
          ))}
        </dl>
      </section>
      <section className="panel">
        <h2>Allowed Assets</h2>
        <p>{Array.isArray(production?.allowed_assets) ? production.allowed_assets.join(", ") : "Pending"}</p>
      </section>
      <section className="panel wide">
        <h2>Production Policy JSON</h2>
        <pre>{JSON.stringify(production ?? policy, null, 2)}</pre>
      </section>
    </main>
  );
}
