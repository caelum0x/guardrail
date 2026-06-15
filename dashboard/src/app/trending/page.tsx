import { getJsonOrNull } from "../../lib/api";

interface TrendingToken {
  rank: number;
  symbol: string;
  cmc_id: number;
}

interface TrendingResponse {
  tokens: TrendingToken[];
  error?: string;
}

function rows(value: TrendingResponse | null): TrendingToken[] {
  return Array.isArray(value?.tokens) ? value.tokens : [];
}

export default async function TrendingPage() {
  const data = await getJsonOrNull<TrendingResponse>("/trending");
  const tokens = rows(data);

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Trending Tokens</h2>
        <p className="eyebrow">
          Trending tokens surfaced by the CMC data source, ranked by momentum.
        </p>
        {data?.error || !data ? (
          <p className="mono">
            Error: {data?.error ?? "Unable to load trending tokens."}
          </p>
        ) : tokens.length === 0 ? (
          <p>No trending tokens available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Rank</th>
                <th>Symbol</th>
                <th>CMC ID</th>
              </tr>
            </thead>
            <tbody>
              {tokens.map((token) => (
                <tr key={token.cmc_id}>
                  <td>{token.rank}</td>
                  <td>{token.symbol}</td>
                  <td>{token.cmc_id}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
