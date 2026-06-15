import { getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface WatchlistAsset {
  symbol: string;
  category: string;
  status: "normal" | "watch" | "critical" | string;
  attention_score: Numeric;
  price_usd: Numeric;
  ret_24h: Numeric;
  volatility_1h: Numeric;
  liquidity_usd: Numeric;
  safety_score: number;
  reasons: string[];
}

interface WatchlistResponse {
  counts: {
    critical: number;
    watch: number;
    total: number;
  };
  fear_greed?: {
    value: number;
    classification: string;
  } | null;
  assets: WatchlistAsset[];
  error?: string;
}

function n(value: Numeric, digits = 2): string {
  if (value === null || value === undefined) {
    return "-";
  }
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return String(value);
  }
  return parsed.toFixed(digits);
}

function pct(value: Numeric): string {
  return `${n(value)}%`;
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
  if (status === "critical") {
    return "critical";
  }
  if (status === "watch") {
    return "warning";
  }
  return "clear";
}

export default async function WatchlistPage() {
  const data = await getJsonOrNull<WatchlistResponse>("/watchlist");
  const assets = Array.isArray(data?.assets) ? data.assets : [];
  const pageStatus =
    (data?.counts.critical ?? 0) > 0
      ? "critical"
      : (data?.counts.watch ?? 0) > 0
        ? "watch"
        : "normal";

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(pageStatus)}`}>
        <div>
          <h2>Watchlist</h2>
          {data?.error ? (
            <p>Failed to load watchlist: {data.error}</p>
          ) : !data ? (
            <p>Watchlist unavailable.</p>
          ) : (
            <p>Enabled non-stable assets ranked by market attention score.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Total</span>
              <strong>{data.counts.total}</strong>
            </div>
            <div>
              <span>Critical</span>
              <strong>{data.counts.critical}</strong>
            </div>
            <div>
              <span>Watch</span>
              <strong>{data.counts.watch}</strong>
            </div>
            <div>
              <span>Fear &amp; Greed</span>
              <strong>{data.fear_greed?.value ?? "-"}</strong>
            </div>
          </div>
        ) : null}
      </section>

      <section className="panel wide">
        <h2>Assets</h2>
        {assets.length === 0 ? (
          <p>No watchlist assets available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Symbol</th>
                <th>Status</th>
                <th>Score</th>
                <th>24h</th>
                <th>Volatility</th>
                <th>Liquidity</th>
                <th>Reason</th>
              </tr>
            </thead>
            <tbody>
              {assets.map((asset) => (
                <tr key={asset.symbol}>
                  <td>{asset.symbol}</td>
                  <td>{label(asset.status)}</td>
                  <td>{n(asset.attention_score)}</td>
                  <td>{pct(asset.ret_24h)}</td>
                  <td>{pct(asset.volatility_1h)}</td>
                  <td>{usd(asset.liquidity_usd)}</td>
                  <td>{asset.reasons.slice(0, 2).join("; ")}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
