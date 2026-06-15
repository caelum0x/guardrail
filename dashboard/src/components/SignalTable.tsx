import type { SignalsResponse } from "../lib/types";

function valueAsText(value: unknown): string {
  if (value === null || value === undefined) {
    return "Pending";
  }
  if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  return JSON.stringify(value);
}

export function SignalTable({ signals }: { signals?: SignalsResponse | null }) {
  const regime = signals?.regime ?? {};
  const target = signals?.target ?? {};

  return (
    <section className="panel">
      <h2>Signals</h2>
      <table>
        <thead>
          <tr>
            <th>Signal</th>
            <th>Value</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>Regime</td>
            <td>{valueAsText(regime.regime)}</td>
          </tr>
          <tr>
            <td>Strategy headline</td>
            <td>{valueAsText(target.headline)}</td>
          </tr>
          <tr>
            <td>Proposed orders</td>
            <td>{valueAsText(target.orders)}</td>
          </tr>
        </tbody>
      </table>
    </section>
  );
}
