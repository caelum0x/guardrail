import { getJsonOrNull } from "../../lib/api";

interface RegimeInputs {
  fear_greed: number;
  breadth_pct: string;
  btc_dominance_pct: string;
  median_24h_return: string;
}

interface RegimeResponse {
  regime: string;
  exposure_multiplier: string;
  inputs: RegimeInputs;
  error?: string;
}

function pct(value: string | null | undefined): string {
  if (value === null || value === undefined) {
    return "—";
  }
  const num = Number(value);
  if (!Number.isFinite(num)) {
    return value;
  }
  return `${num.toFixed(2)}%`;
}

function num(value: string | null | undefined, digits = 2): string {
  if (value === null || value === undefined) {
    return "—";
  }
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return value;
  }
  return parsed.toFixed(digits);
}

function regimeLabel(regime: string): string {
  return regime
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export default async function RegimePage() {
  const data = await getJsonOrNull<RegimeResponse>("/regime");
  const inputs = data?.inputs ?? null;

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Market Regime</h2>
        {data?.error ? (
          <p>Failed to load regime: {data.error}</p>
        ) : !data ? (
          <p>Regime unavailable.</p>
        ) : (
          <div className="metricGrid">
            <div>
              <span>Regime</span>
              <strong>{regimeLabel(data.regime)}</strong>
            </div>
            <div>
              <span>Exposure Multiplier</span>
              <strong>{num(data.exposure_multiplier)}x</strong>
            </div>
          </div>
        )}
      </section>
      {inputs ? (
        <section className="panel wide">
          <h2>Regime Inputs</h2>
          <div className="metricGrid">
            <div>
              <span>Fear &amp; Greed</span>
              <strong>{inputs.fear_greed}</strong>
            </div>
            <div>
              <span>Breadth</span>
              <strong>{pct(inputs.breadth_pct)}</strong>
            </div>
            <div>
              <span>BTC Dominance</span>
              <strong>{pct(inputs.btc_dominance_pct)}</strong>
            </div>
            <div>
              <span>Median 24h Return</span>
              <strong>{pct(inputs.median_24h_return)}</strong>
            </div>
          </div>
        </section>
      ) : null}
    </main>
  );
}
