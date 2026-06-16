import { getJsonOrNull } from "../../lib/api";

interface CorrelationResponse {
  spec?: string;
  names?: string[];
  matrix?: number[][];
  error?: string;
}

/** Background tint from a correlation value in [-1, 1]: green positive, red negative. */
function tint(v: number): string {
  const a = Math.min(Math.abs(v), 1);
  if (v >= 0) return `rgba(40, 167, 69, ${0.15 + a * 0.5})`;
  return `rgba(220, 53, 69, ${0.15 + a * 0.5})`;
}

export default async function CorrelationPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | undefined>>;
}) {
  const p = await searchParams;
  const qs = p.series ? `?series=${encodeURIComponent(p.series)}` : "";
  const data = await getJsonOrNull<CorrelationResponse>(`/correlation${qs}`);
  const names = data?.names ?? [];
  const matrix = data?.matrix ?? [];

  return (
    <main className="grid">
      <section className="card">
        <h1>Correlation Matrix</h1>
        <p>
          Pairwise Pearson correlation over named return series, via the{" "}
          <code>correlation</code> crate (<code>GET /correlation</code>). Drive it
          with <code>?series=BTC:0.01,-0.02,0.03;ETH:0.012,-0.018,0.025</code>.
        </p>
        {data?.error ? <p>⚠️ {data.error}</p> : null}
      </section>

      {names.length > 0 && matrix.length > 0 ? (
        <section className="card">
          <table>
            <thead>
              <tr>
                <th></th>
                {names.map((n) => (
                  <th key={n}>{n}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {matrix.map((row, i) => (
                <tr key={names[i]}>
                  <th>{names[i]}</th>
                  {row.map((v, j) => (
                    <td key={j} style={{ backgroundColor: tint(v), textAlign: "right" }}>
                      {v.toFixed(2)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      ) : null}
    </main>
  );
}
