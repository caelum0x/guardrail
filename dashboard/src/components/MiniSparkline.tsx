/**
 * Compact, self-contained SVG NAV sparkline for the cockpit home page.
 * Mirrors the equity/backtest sparkline approach in a smaller inline form.
 */
interface MiniSparklineProps {
  values: number[];
  height?: number;
}

export function MiniSparkline({ values, height = 80 }: MiniSparklineProps) {
  if (values.length < 2) {
    return <p className="mono">Not enough NAV history to plot.</p>;
  }

  const width = 360;
  const pad = 6;
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = max - min || 1;
  const points = values
    .map((value, index) => {
      const x = pad + (index / (values.length - 1)) * (width - 2 * pad);
      const y = height - pad - ((value - min) / span) * (height - 2 * pad);
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
  const up = values[values.length - 1] >= values[0];
  const stroke = up ? "#22c55e" : "#ef4444";

  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      width="100%"
      height={height}
      role="img"
      aria-label="NAV equity curve"
    >
      <polyline points={points} fill="none" stroke={stroke} strokeWidth="2" />
    </svg>
  );
}
