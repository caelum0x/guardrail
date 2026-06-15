import { getJsonOrNull } from "../../lib/api";

interface FundingRow {
  symbol: string;
  price_usd: string;
  ret_24h: string | null;
  funding_rate_proxy: string;
}

interface FundingResponse {
  assets: FundingRow[];
  error?: string;
}

function rows(value: FundingResponse | null): FundingRow[] {
  return Array.isArray(value?.assets) ? value.assets : [];
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

function rate(value: string): string {
  const num = Number(value);
  if (!Number.isFinite(num)) {
    return value;
  }
  return num.toFixed(4);
}

export default async function FundingPage() {
  const data = await getJsonOrNull<FundingResponse>("/funding");
  const assets = rows(data);

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Funding Proxy</h2>
        <p>
          Synthetic per-hour funding-rate proxy that approximates perpetual-swap
          funding pressure for regime rotation. Derived from 24h return and 1h
          volatility on mock market data — not a live derivatives feed.
        </p>
        {data?.error ? (
          <p>Failed to load funding: {data.error}</p>
        ) : assets.length === 0 ? (
          <p>No funding data available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Symbol</th>
                <th>24h%</th>
                <th>Funding Proxy</th>
              </tr>
            </thead>
            <tbody>
              {assets.map((asset) => (
                <tr key={asset.symbol}>
                  <td>{asset.symbol}</td>
                  <td>{pct(asset.ret_24h)}</td>
                  <td>{rate(asset.funding_rate_proxy)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
