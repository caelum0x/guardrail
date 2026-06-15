import { getJsonOrNull } from "../../lib/api";

interface OptimizeMethods {
  equal_weight: number[];
  score_proportional: number[];
  inverse_volatility: number[];
  risk_parity: number[];
}

interface OptimizeResponse {
  symbols: string[];
  scores: number[];
  vols: number[];
  methods: OptimizeMethods;
  error?: string;
}

const METHODS: { key: keyof OptimizeMethods; label: string }[] = [
  { key: "equal_weight", label: "Equal Weight" },
  { key: "score_proportional", label: "Score Proportional" },
  { key: "inverse_volatility", label: "Inverse Volatility" },
  { key: "risk_parity", label: "Risk Parity" },
];

/** Format a unit-budget weight as a percentage, or an em dash if missing. */
function pct(value: number | undefined): string {
  if (value === undefined || !Number.isFinite(value)) {
    return "—";
  }
  return `${(value * 100).toFixed(2)}%`;
}

export default async function OptimizerPage() {
  const data = await getJsonOrNull<OptimizeResponse>("/optimize");
  const symbols = Array.isArray(data?.symbols) ? data.symbols : [];

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Portfolio Optimizer</h2>
        <p className="eyebrow">
          Allocation weights for an example basket across four methods. Weights
          are unit-budget fractions of the portfolio.
        </p>
        {data?.error || !data ? (
          <p className="mono">
            Error: {data?.error ?? "Unable to load optimizer."}
          </p>
        ) : symbols.length === 0 ? (
          <p>No allocation available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Symbol</th>
                <th>Score</th>
                <th>Vol</th>
                {METHODS.map((m) => (
                  <th key={m.key}>{m.label}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {symbols.map((symbol, i) => (
                <tr key={symbol}>
                  <td>{symbol}</td>
                  <td>{data.scores?.[i] ?? "—"}</td>
                  <td>{data.vols?.[i] ?? "—"}</td>
                  {METHODS.map((m) => (
                    <td key={m.key}>{pct(data.methods?.[m.key]?.[i])}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
