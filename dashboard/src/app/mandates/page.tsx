import { getJsonOrNull } from "../../lib/api";

interface Mandate {
  id: string;
  label: string;
  mandate: string;
  policy_hash: string;
  policy: {
    max_total_drawdown_pct: string;
    max_daily_drawdown_pct: string;
    max_position_pct: string;
    max_new_position_pct: string;
    min_stable_reserve_pct: string;
    max_slippage_pct: string;
    execution_layer: string;
    require_quote_before_swap: boolean;
    daily_trade_enabled: boolean;
    min_trades_per_day: number;
  };
}

interface MandatesResponse {
  count: number;
  mandates: Mandate[];
  error?: string;
}

function pct(value: string): string {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return value;
  }
  return `${parsed.toFixed(2)}%`;
}

export default async function MandatesPage() {
  const data = await getJsonOrNull<MandatesResponse>("/mandates");
  const mandates = Array.isArray(data?.mandates) ? data.mandates : [];

  return (
    <main className="grid">
      <section className="panel wide statusPanel clear">
        <div>
          <h2>Mandates</h2>
          {data?.error ? (
            <p>Failed to load mandates: {data.error}</p>
          ) : !data ? (
            <p>Mandates unavailable.</p>
          ) : (
            <p>Natural-language mandates compiled into policy hashes.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Mandates</span>
              <strong>{data.count}</strong>
            </div>
            <div>
              <span>Execution</span>
              <strong>twak_only</strong>
            </div>
            <div>
              <span>Quote Required</span>
              <strong>true</strong>
            </div>
          </div>
        ) : null}
      </section>

      {mandates.map((mandate) => (
        <section className="panel wide" key={mandate.id}>
          <h2>{mandate.label}</h2>
          <p>{mandate.mandate}</p>
          <div className="metricGrid">
            <div>
              <span>Policy Hash</span>
              <strong className="mono">{mandate.policy_hash}</strong>
            </div>
            <div>
              <span>Total Drawdown</span>
              <strong>{pct(mandate.policy.max_total_drawdown_pct)}</strong>
            </div>
            <div>
              <span>Position Cap</span>
              <strong>{pct(mandate.policy.max_position_pct)}</strong>
            </div>
            <div>
              <span>Stable Reserve</span>
              <strong>{pct(mandate.policy.min_stable_reserve_pct)}</strong>
            </div>
          </div>
        </section>
      ))}
    </main>
  );
}
