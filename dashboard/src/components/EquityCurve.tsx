/**
 * Dependency-free, presentational equity-curve line chart rendered as inline
 * SVG. Accepts either a numeric series or a list of `{ nav }` string points so
 * it can be fed directly from `/history` (NAV points) or a backtest
 * `equity_curve` (string array) without callers reshaping their data.
 */

export type NavPoint = { nav: string };

export interface EquityCurveProps {
  /** Series to plot: raw numbers, or objects carrying a `nav` string. */
  points: number[] | NavPoint[];
  /** Pixel height of the chart (width is fluid at 100%). */
  height?: number;
  /** Accessible label for the rendered chart. */
  label?: string;
}

const WIDTH = 720;
const PAD = 8;

/** Coerce mixed `number | { nav }` input into a finite numeric series. */
function toValues(points: number[] | NavPoint[]): number[] {
  return points
    .map((point) =>
      typeof point === "number" ? point : Number(point.nav),
    )
    .filter((value) => Number.isFinite(value));
}

/** Build the SVG polyline point string for a finite series. */
function buildPolyline(values: number[], height: number): string {
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = max - min || 1;
  return values
    .map((value, index) => {
      const x = PAD + (index / (values.length - 1)) * (WIDTH - 2 * PAD);
      const y =
        height - PAD - ((value - min) / span) * (height - 2 * PAD);
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

export function EquityCurve({
  points,
  height = 180,
  label = "Equity curve",
}: EquityCurveProps) {
  const values = toValues(points);

  if (values.length < 2) {
    return <p className="mono">Not enough data to plot.</p>;
  }

  const polyline = buildPolyline(values, height);
  const up = values[values.length - 1] >= values[0];
  const stroke = up ? "#22c55e" : "#ef4444";

  return (
    <svg
      viewBox={`0 0 ${WIDTH} ${height}`}
      width="100%"
      height={height}
      role="img"
      aria-label={label}
    >
      <polyline points={polyline} fill="none" stroke={stroke} strokeWidth="2" />
    </svg>
  );
}
