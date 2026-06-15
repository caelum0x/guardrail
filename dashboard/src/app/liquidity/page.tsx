import { getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface LiquidityAsset {
  symbol: string;
  category: string;
  status: string;
  liquidity_usd: Numeric;
  capacity_usd: Numeric;
  order_notional_usd: Numeric;
  pool_usage_pct: Numeric;
  headroom_usd: Numeric;
  safety_score: number;
}

interface LiquidityResponse {
  summary: {
    assets: number;
    blocking: number;
    watch: number;
    ok: number;
  };
  thresholds: {
    max_pool_usage_pct: Numeric;
    warning_pool_usage_pct: Numeric;
    min_liquidity_usd: Numeric;
    default_order_notional_usd: Numeric;
  };
  assets: LiquidityAsset[];
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

function pct(value: Numeric): string {
  return `${n(value, 4)}%`;
}

function label(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function statusClass(status: string): string {
  if (status === "blocking") return "critical";
  if (status === "watch") return "warning";
  return "clear";
}

export default async function LiquidityPage() {
  const data = await getJsonOrNull<LiquidityResponse>("/liquidity");
  const assets = Array.isArray(data?.assets) ? data.assets : [];
  const pageStatus =
    (data?.summary.blocking ?? 0) > 0
      ? "blocking"
      : (data?.summary.watch ?? 0) > 0
        ? "watch"
        : "ok";

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(pageStatus)}`}>
        <div>
          <h2>Liquidity</h2>
          {data?.error ? (
            <p>Failed to load liquidity: {data.error}</p>
          ) : !data ? (
            <p>Liquidity unavailable.</p>
          ) : (
            <p>Configured order notional compared with current DEX liquidity.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Assets</span>
              <strong>{data.summary.assets}</strong>
            </div>
            <div>
              <span>Blocking</span>
              <strong>{data.summary.blocking}</strong>
            </div>
            <div>
              <span>Watch</span>
              <strong>{data.summary.watch}</strong>
            </div>
            <div>
              <span>Max Pool Use</span>
              <strong>{pct(data.thresholds.max_pool_usage_pct)}</strong>
            </div>
          </div>
        ) : null}
      </section>

      <section className="panel wide">
        <h2>Assets</h2>
        {assets.length === 0 ? (
          <p>No liquidity rows available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Symbol</th>
                <th>Status</th>
                <th>Liquidity</th>
                <th>Capacity</th>
                <th>Usage</th>
                <th>Headroom</th>
              </tr>
            </thead>
            <tbody>
              {assets.map((asset) => (
                <tr key={asset.symbol}>
                  <td>{asset.symbol}</td>
                  <td>{label(asset.status)}</td>
                  <td>{usd(asset.liquidity_usd)}</td>
                  <td>{usd(asset.capacity_usd)}</td>
                  <td>{pct(asset.pool_usage_pct)}</td>
                  <td>{usd(asset.headroom_usd)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
