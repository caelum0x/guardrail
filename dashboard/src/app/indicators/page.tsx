import { getJsonOrNull } from "../../lib/api";

type Macd = {
  macd: number[];
  signal: number[];
  histogram: number[];
};

type Bollinger = {
  mid: number[];
  upper: number[];
  lower: number[];
};

type IndicatorsResponse = {
  symbol: string;
  steps: number;
  closes: number[];
  ema: number[];
  sma: number[];
  rsi: number[];
  macd: Macd;
  bollinger: Bollinger;
  error?: string;
};

const SYMBOLS = ["WBNB", "CAKE", "USDT"];

/** Latest finite value of a series, or null when the series is empty. */
function last(values: number[] | undefined): number | null {
  if (!values || values.length === 0) {
    return null;
  }
  const v = values[values.length - 1];
  return Number.isFinite(v) ? v : null;
}

/** Format a numeric metric, or an em dash when unavailable. */
function fmt(value: number | null, digits = 4): string {
  return value === null ? "—" : value.toFixed(digits);
}

type Line = {
  values: number[];
  stroke: string;
  width: number;
  label: string;
};

/**
 * Render the close series with the Bollinger upper/lower bands as faint
 * polylines. All series share a single min/max scale so the bands line up
 * with the price. Reuses the sparkline approach from the backtest page.
 */
function PriceChart({
  closes,
  upper,
  lower,
}: {
  closes: number[];
  upper: number[];
  lower: number[];
}) {
  if (closes.length < 2) {
    return <p>Not enough data to plot.</p>;
  }
  const width = 720;
  const height = 220;
  const pad = 8;

  const lines: Line[] = [
    { values: upper, stroke: "#94a3b8", width: 1, label: "Bollinger upper" },
    { values: lower, stroke: "#94a3b8", width: 1, label: "Bollinger lower" },
    { values: closes, stroke: "#38bdf8", width: 2, label: "Close" },
  ];

  const scale = [closes, upper, lower]
    .flat()
    .filter((n) => Number.isFinite(n));
  const min = scale.length > 0 ? Math.min(...scale) : 0;
  const max = scale.length > 0 ? Math.max(...scale) : 1;
  const span = max - min || 1;

  const toPoints = (values: number[]): string =>
    values
      .map((v, i) => {
        const x = pad + (i / (closes.length - 1)) * (width - 2 * pad);
        const y = height - pad - ((v - min) / span) * (height - 2 * pad);
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");

  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      width="100%"
      height={height}
      role="img"
      aria-label="Close price with Bollinger Bands"
    >
      {lines.map((line) =>
        line.values.length >= 2 ? (
          <polyline
            key={line.label}
            points={toPoints(line.values)}
            fill="none"
            stroke={line.stroke}
            strokeWidth={line.width}
            opacity={line.width === 1 ? 0.5 : 1}
          />
        ) : null,
      )}
    </svg>
  );
}

export default async function IndicatorsPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | undefined>>;
}) {
  const params = await searchParams;
  const symbol = params.symbol ?? "WBNB";
  const steps = params.steps ?? "60";

  const data = await getJsonOrNull<IndicatorsResponse>(
    `/indicators?symbol=${encodeURIComponent(symbol)}&steps=${encodeURIComponent(steps)}`,
  );

  const closes = data?.closes ?? [];
  const macdLatest = last(data?.macd?.macd);
  const signalLatest = last(data?.macd?.signal);
  const histLatest = last(data?.macd?.histogram);
  const bbUpper = last(data?.bollinger?.upper);
  const bbMid = last(data?.bollinger?.mid);
  const bbLower = last(data?.bollinger?.lower);

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Technical Indicators</h2>
        <p className="eyebrow">
          Classic indicators (EMA, SMA, RSI, MACD, Bollinger Bands) over a
          deterministic synthetic close series.
        </p>
        <div className="actions">
          {SYMBOLS.map((s) => (
            <a
              key={s}
              className="buttonLink"
              href={`/indicators?symbol=${encodeURIComponent(s)}&steps=${encodeURIComponent(steps)}`}
            >
              {s}
            </a>
          ))}
        </div>
        {data?.error || !data ? (
          <p className="mono">
            Error: {data?.error ?? "Unable to load indicators."}
          </p>
        ) : (
          <div className="metricGrid">
            <div>
              <span>Symbol</span>
              <strong>{data.symbol}</strong>
            </div>
            <div>
              <span>Steps</span>
              <strong>{data.steps}</strong>
            </div>
            <div>
              <span>Close</span>
              <strong>{fmt(last(data.closes))}</strong>
            </div>
            <div>
              <span>EMA (12)</span>
              <strong>{fmt(last(data.ema))}</strong>
            </div>
            <div>
              <span>SMA (12)</span>
              <strong>{fmt(last(data.sma))}</strong>
            </div>
            <div>
              <span>RSI (14)</span>
              <strong>{fmt(last(data.rsi), 2)}</strong>
            </div>
            <div>
              <span>MACD</span>
              <strong>{fmt(macdLatest)}</strong>
            </div>
            <div>
              <span>Signal</span>
              <strong>{fmt(signalLatest)}</strong>
            </div>
            <div>
              <span>Histogram</span>
              <strong>{fmt(histLatest)}</strong>
            </div>
            <div>
              <span>Bollinger upper</span>
              <strong>{fmt(bbUpper)}</strong>
            </div>
            <div>
              <span>Bollinger mid</span>
              <strong>{fmt(bbMid)}</strong>
            </div>
            <div>
              <span>Bollinger lower</span>
              <strong>{fmt(bbLower)}</strong>
            </div>
          </div>
        )}
      </section>
      <section className="panel wide">
        <h2>Close &amp; Bollinger Bands</h2>
        <PriceChart
          closes={closes}
          upper={data?.bollinger?.upper ?? []}
          lower={data?.bollinger?.lower ?? []}
        />
      </section>
    </main>
  );
}
