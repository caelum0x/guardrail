import { getJsonOrNull } from "../../lib/api";

interface FearGreed {
  value: number;
  classification: string;
}

interface AssetRow {
  symbol: string;
  price_usd: string;
  ret_24h: string | null;
  volume_24h_usd: string;
  liquidity_usd: string | null;
  safety_score: number;
  category: string;
}

interface AssetsResponse {
  fear_greed: FearGreed | null;
  assets: AssetRow[];
  error?: string;
}

function rows(value: AssetsResponse | null): AssetRow[] {
  return Array.isArray(value?.assets) ? value.assets : [];
}

function usd(value: string | null): string {
  if (value === null) {
    return "—";
  }
  const num = Number(value);
  if (!Number.isFinite(num)) {
    return value;
  }
  return `$${num.toLocaleString(undefined, { maximumFractionDigits: 2 })}`;
}

function pct(value: string | null): string {
  if (value === null) {
    return "—";
  }
  const num = Number(value);
  if (!Number.isFinite(num)) {
    return value;
  }
  return `${num.toFixed(2)}%`;
}

export default async function AssetsPage() {
  const data = await getJsonOrNull<AssetsResponse>("/assets");
  const assets = rows(data);
  const fg = data?.fear_greed ?? null;

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Assets Overview</h2>
        <div className="metricGrid">
          <div>
            <span>Fear &amp; Greed</span>
            <strong>
              {fg ? `${fg.value} · ${fg.classification}` : "Unavailable"}
            </strong>
          </div>
          <div>
            <span>Assets</span>
            <strong>{assets.length}</strong>
          </div>
        </div>
      </section>
      <section className="panel wide">
        {data?.error ? (
          <p>Failed to load assets: {data.error}</p>
        ) : assets.length === 0 ? (
          <p>No assets available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Symbol</th>
                <th>Price</th>
                <th>24h%</th>
                <th>Volume</th>
                <th>Liquidity</th>
                <th>Safety</th>
                <th>Category</th>
              </tr>
            </thead>
            <tbody>
              {assets.map((asset) => (
                <tr key={asset.symbol}>
                  <td>{asset.symbol}</td>
                  <td>{usd(asset.price_usd)}</td>
                  <td>{pct(asset.ret_24h)}</td>
                  <td>{usd(asset.volume_24h_usd)}</td>
                  <td>{usd(asset.liquidity_usd)}</td>
                  <td>{asset.safety_score}</td>
                  <td>{asset.category}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
