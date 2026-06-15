import type { PortfolioResponse } from "../lib/types";
import { usdString } from "../lib/format";

export function PortfolioTable({ portfolio }: { portfolio?: PortfolioResponse | null }) {
  const latest = portfolio?.latest;

  return (
    <section className="panel">
      <h2>Portfolio</h2>
      <table>
        <thead>
          <tr>
            <th>Metric</th>
            <th>Value</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>Net asset value</td>
            <td>{usdString(latest?.nav_usd)}</td>
          </tr>
          <tr>
            <td>Open positions</td>
            <td>{latest?.positions ?? 0}</td>
          </tr>
          <tr>
            <td>Source event</td>
            <td className="mono">{portfolio?.source_event_id ?? "Pending"}</td>
          </tr>
        </tbody>
      </table>
    </section>
  );
}
