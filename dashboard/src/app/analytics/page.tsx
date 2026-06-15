import { getJsonOrNull } from "../../lib/api";
import { EquityCurve } from "../../components/EquityCurve";
import type {
  CostsResponse,
  ExposureResponse,
  HistoryResponse,
  Numeric,
  RegimeResponse,
} from "../../lib/types";

/** Format a number as a fixed-precision string, with a dash for non-finite input. */
function n(value: Numeric, digits = 2): string {
  if (value === null || value === undefined) {
    return "—";
  }
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return String(value);
  }
  return parsed.toFixed(digits);
}

function usd(value: Numeric, digits = 2): string {
  if (value === null || value === undefined) {
    return "—";
  }
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return String(value);
  }
  return `$${parsed.toLocaleString("en-US", {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  })}`;
}

function pct(value: Numeric, digits = 2): string {
  return `${n(value, digits)}%`;
}

function titleCase(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

type DrawdownSummary = {
  maxDrawdownPct: number;
  currentDrawdownPct: number;
  peakNav: number;
  troughNav: number;
};

/** Compute peak-to-trough drawdown statistics from a NAV series. */
function computeDrawdown(navValues: number[]): DrawdownSummary | null {
  if (navValues.length < 2) {
    return null;
  }
  let peak = navValues[0];
  let troughNav = navValues[0];
  let maxDrawdownPct = 0;
  for (const nav of navValues) {
    if (nav > peak) {
      peak = nav;
    }
    if (peak > 0) {
      const drawdown = ((peak - nav) / peak) * 100;
      if (drawdown > maxDrawdownPct) {
        maxDrawdownPct = drawdown;
        troughNav = nav;
      }
    }
  }
  const last = navValues[navValues.length - 1];
  const currentDrawdownPct = peak > 0 ? ((peak - last) / peak) * 100 : 0;
  return { maxDrawdownPct, currentDrawdownPct, peakNav: peak, troughNav };
}

export default async function AnalyticsPage() {
  const [history, regime, exposure, costs] = await Promise.all([
    getJsonOrNull<HistoryResponse>("/history"),
    getJsonOrNull<RegimeResponse>("/regime"),
    getJsonOrNull<ExposureResponse>("/exposure"),
    getJsonOrNull<CostsResponse>("/costs"),
  ]);

  const navValues = (history?.points ?? [])
    .map((point) => Number(point.nav_usd))
    .filter((value) => Number.isFinite(value));
  const hasSeries = navValues.length > 0;
  const firstNav = hasSeries ? navValues[0] : null;
  const lastNav = hasSeries ? navValues[navValues.length - 1] : null;
  const changePct =
    firstNav !== null && lastNav !== null && firstNav !== 0
      ? ((lastNav - firstNav) / firstNav) * 100
      : null;
  const drawdown = computeDrawdown(navValues);

  const exposurePositions = Array.isArray(exposure?.positions)
    ? exposure.positions
    : [];

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Analytics Overview</h2>
        <p className="eyebrow">
          Consolidated read-only view of NAV performance, drawdown, market regime,
          per-asset exposure, and execution costs.
        </p>
      </section>

      <section className="panel wide">
        <h2>NAV Equity Curve</h2>
        {history?.error ? (
          <p className="mono">Error: {history.error}</p>
        ) : !hasSeries ? (
          <p className="mono">No NAV history available yet.</p>
        ) : (
          <>
            <div className="metricGrid">
              <div>
                <span>First NAV</span>
                <strong>{usd(firstNav)}</strong>
              </div>
              <div>
                <span>Latest NAV</span>
                <strong>{usd(lastNav)}</strong>
              </div>
              <div>
                <span>Change</span>
                <strong>{changePct !== null ? pct(changePct) : "—"}</strong>
              </div>
              <div>
                <span>Points</span>
                <strong>{history?.count ?? navValues.length}</strong>
              </div>
            </div>
            <EquityCurve points={navValues} label="NAV equity curve" />
          </>
        )}
      </section>

      <section className="panel wide">
        <h2>Drawdown</h2>
        {!drawdown ? (
          <p className="mono">Not enough NAV history to compute drawdown.</p>
        ) : (
          <div className="metricGrid">
            <div>
              <span>Max Drawdown</span>
              <strong>{pct(drawdown.maxDrawdownPct)}</strong>
            </div>
            <div>
              <span>Current Drawdown</span>
              <strong>{pct(drawdown.currentDrawdownPct)}</strong>
            </div>
            <div>
              <span>Peak NAV</span>
              <strong>{usd(drawdown.peakNav)}</strong>
            </div>
            <div>
              <span>Trough NAV</span>
              <strong>{usd(drawdown.troughNav)}</strong>
            </div>
          </div>
        )}
      </section>

      <section className="panel wide">
        <h2>Regime &amp; Exposure</h2>
        {regime?.error ? (
          <p className="mono">Failed to load regime: {regime.error}</p>
        ) : !regime ? (
          <p className="mono">Regime unavailable.</p>
        ) : (
          <div className="metricGrid">
            <div>
              <span>Regime</span>
              <strong>{titleCase(regime.regime)}</strong>
            </div>
            <div>
              <span>Exposure Multiplier</span>
              <strong>{n(regime.exposure_multiplier)}x</strong>
            </div>
            <div>
              <span>Exposure Status</span>
              <strong>
                {exposure && !exposure.error
                  ? titleCase(exposure.status)
                  : "—"}
              </strong>
            </div>
            <div>
              <span>Portfolio NAV</span>
              <strong>
                {exposure && !exposure.error ? usd(exposure.nav_usd) : "—"}
              </strong>
            </div>
          </div>
        )}
      </section>

      <section className="panel wide">
        <h2>Per-Asset Exposure</h2>
        {exposure?.error ? (
          <p className="mono">Failed to load exposure: {exposure.error}</p>
        ) : exposurePositions.length === 0 ? (
          <p className="mono">No positions available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Symbol</th>
                <th>Category</th>
                <th>Weight</th>
                <th>Value</th>
              </tr>
            </thead>
            <tbody>
              {exposurePositions.map((position) => (
                <tr key={position.symbol}>
                  <td>{position.symbol}</td>
                  <td>{titleCase(position.category)}</td>
                  <td>{pct(position.weight_pct)}</td>
                  <td>{usd(position.value_usd)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section className="panel wide">
        <h2>Execution Cost Summary</h2>
        {costs?.error ? (
          <p className="mono">Failed to load costs: {costs.error}</p>
        ) : !costs ? (
          <p className="mono">Execution costs unavailable.</p>
        ) : (
          <div className="metricGrid">
            <div>
              <span>Chain</span>
              <strong>{String(costs.chain).toUpperCase()}</strong>
            </div>
            <div>
              <span>Routes</span>
              <strong>{costs.summary.routes}</strong>
            </div>
            <div>
              <span>Total Cost</span>
              <strong>{usd(costs.summary.total_all_in_cost_usd, 4)}</strong>
            </div>
            <div>
              <span>Average BPS</span>
              <strong>{n(costs.summary.average_cost_bps)}</strong>
            </div>
          </div>
        )}
      </section>
    </main>
  );
}
