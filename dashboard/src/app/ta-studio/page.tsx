import { getJsonOrNull } from "../../lib/api";
import {
  isTaSuccess,
  toColumns,
  TA_INDICATORS,
  type TaIndicator,
  type TaResponse,
  type TaValue,
} from "../../lib/types-ta";

/** Default close-price series used when the URL carries no `series` param. */
const DEFAULT_SERIES =
  "44,44.34,44.09,44.15,43.61,44.33,44.83,45.10,45.42,45.84,46.08,45.89,46.03,45.61,46.28,46.28,46.00,46.03,46.41,46.22,45.64,46.21,46.25,45.71,46.45,45.78,45.35,44.03,44.18,44.22";

const DEFAULT_INDICATOR: TaIndicator = "rsi";
const DEFAULT_PERIOD = 14;
const DEFAULT_MULT = 2.0;

/** Human-readable one-liners for each supported indicator. */
const INDICATOR_DESCRIPTIONS: Record<TaIndicator, string> = {
  sma: "Simple moving average over the lookback period.",
  ema: "Exponential moving average, weighting recent closes more heavily.",
  rsi: "Relative Strength Index (0–100 momentum oscillator).",
  macd: "Moving Average Convergence Divergence (12/26/9 fixed periods).",
  bollinger: "Bollinger Bands: SMA midline ± mult × standard deviation.",
};

/** Narrow a raw query value to a supported indicator, falling back to default. */
function parseIndicator(raw: string | undefined): TaIndicator {
  const candidate = (raw ?? "").toLowerCase();
  return (TA_INDICATORS as readonly string[]).includes(candidate)
    ? (candidate as TaIndicator)
    : DEFAULT_INDICATOR;
}

/** Parse a positive integer query value, falling back to a default. */
function parsePeriod(raw: string | undefined): number {
  const n = Number.parseInt(raw ?? "", 10);
  return Number.isFinite(n) && n > 0 ? n : DEFAULT_PERIOD;
}

/** Parse a finite multiplier query value, falling back to a default. */
function parseMult(raw: string | undefined): number {
  const n = Number.parseFloat(raw ?? "");
  return Number.isFinite(n) && n > 0 ? n : DEFAULT_MULT;
}

/** Format a single indicator value, mapping JSON `null` warmup to an em dash. */
function fmt(value: TaValue, digits = 4): string {
  return value === null ? "—" : value.toFixed(digits);
}

/** Parse the comma-separated series for display (mirrors server parsing). */
function parseSeriesForDisplay(raw: string): number[] {
  return raw
    .split(",")
    .map((s) => Number.parseFloat(s.trim()))
    .filter((n) => Number.isFinite(n));
}

export default async function TaStudioPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | undefined>>;
}) {
  const params = await searchParams;

  const indicator = parseIndicator(params.indicator);
  const series = params.series ?? DEFAULT_SERIES;
  const period = parsePeriod(params.period);
  const mult = parseMult(params.mult);

  const query = new URLSearchParams({
    indicator,
    series,
    period: String(period),
    mult: String(mult),
  });

  const data = await getJsonOrNull<TaResponse>(`/ta?${query.toString()}`);
  const closes = parseSeriesForDisplay(series);
  const success = isTaSuccess(data);
  const columns = success ? toColumns(data) : [];

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>TA Studio</h2>
        <p className="eyebrow">
          Compute a technical indicator over an arbitrary close-price series via
          the read-only <code>GET /ta</code> endpoint. Warmup positions return
          JSON <code>null</code> and render as an em dash.
        </p>

        <div className="actions">
          {TA_INDICATORS.map((ind) => {
            const q = new URLSearchParams({
              indicator: ind,
              series,
              period: String(period),
              mult: String(mult),
            });
            return (
              <a
                key={ind}
                className="buttonLink"
                href={`/ta-studio?${q.toString()}`}
                aria-current={ind === indicator ? "page" : undefined}
              >
                {ind.toUpperCase()}
              </a>
            );
          })}
        </div>

        <div className="metricGrid">
          <div>
            <span>Indicator</span>
            <strong>{indicator.toUpperCase()}</strong>
          </div>
          <div>
            <span>Period</span>
            <strong>{period}</strong>
          </div>
          <div>
            <span>Mult (Bollinger)</span>
            <strong>{mult}</strong>
          </div>
          <div>
            <span>Input length</span>
            <strong>{closes.length}</strong>
          </div>
        </div>

        <p className="eyebrow">{INDICATOR_DESCRIPTIONS[indicator]}</p>
      </section>

      <section className="panel wide">
        <h2>Available Indicators</h2>
        <ul>
          {TA_INDICATORS.map((ind) => (
            <li key={ind}>
              <code>{ind}</code> — {INDICATOR_DESCRIPTIONS[ind]}
            </li>
          ))}
        </ul>
      </section>

      <section className="panel wide">
        <h2>Indicator Values</h2>
        {!success ? (
          <p className="mono">
            Error:{" "}
            {data?.error ??
              "Unable to load indicator values (is the API running?)."}
          </p>
        ) : closes.length === 0 ? (
          <p className="mono">No input series to compute over.</p>
        ) : (
          <div style={{ overflowX: "auto" }}>
            <table>
              <thead>
                <tr>
                  <th>#</th>
                  <th>Close</th>
                  {columns.map((col) => (
                    <th key={col.label}>{col.label}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {closes.map((close, i) => (
                  <tr key={i}>
                    <td>{i}</td>
                    <td className="mono">{close.toFixed(4)}</td>
                    {columns.map((col) => (
                      <td key={col.label} className="mono">
                        {fmt(col.values[i] ?? null)}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </main>
  );
}
