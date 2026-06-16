import Link from "next/link";
import { getJsonOrNull } from "../../lib/api";

interface CapsSummary {
  summary?: { cmc_datasets?: number; exposed_capabilities?: number };
}

interface Tool {
  href: string;
  title: string;
  blurb: string;
  api: string;
  example: string;
}

const TOOLS: Tool[] = [
  {
    href: "/ta-studio",
    title: "TA Studio",
    blurb: "Technical indicators (SMA/EMA/RSI/MACD/Bollinger) over a price series.",
    api: "GET /ta",
    example: "/ta-studio?indicator=rsi&period=14",
  },
  {
    href: "/fees",
    title: "Swap Cost",
    blurb: "All-in swap cost: gas + price impact + slippage + protocol fee.",
    api: "GET /fees",
    example: "/fees?notional_usd=25000&quantity=12&side=buy",
  },
  {
    href: "/sizer",
    title: "Position Sizer",
    blurb: "Fixed-fractional, volatility-target, and Kelly position sizing.",
    api: "GET /sizer",
    example: "/sizer?method=kelly&win_prob=0.6&odds=1.5",
  },
  {
    href: "/orderbook",
    title: "Order Book",
    blurb: "Price-time-priority matching engine over a compact order spec.",
    api: "GET /orderbook",
    example: "/orderbook?orders=b,limit,100,5;s,market,,3",
  },
  {
    href: "/pnl",
    title: "PnL Attribution",
    blurb: "Average-cost realized/unrealized PnL per symbol from a fill stream.",
    api: "GET /pnl",
    example: "/pnl?fills=CAKE,buy,10,2;CAKE,sell,4,3&marks=CAKE:3",
  },
];

export default async function QuantPage() {
  const caps = await getJsonOrNull<CapsSummary>("/cmc/capabilities");

  return (
    <main className="grid">
      <section className="card">
        <h1>Quant Tools</h1>
        <p>
          The quant suite — each tool is a real Rust crate exposed as a read-only
          API route, a dashboard page, an SDK method (TS/Python/Go), and a CLI
          subcommand. All computation is pure and offline-safe.
        </p>
        {caps?.summary ? (
          <p>
            Backed by <strong>{caps.summary.cmc_datasets}</strong> CMC datasets →{" "}
            <strong>{caps.summary.exposed_capabilities}</strong> capabilities
            (see <Link href="/market-oracle">market-oracle</Link>).
          </p>
        ) : null}
      </section>

      {TOOLS.map((t) => (
        <section className="card" key={t.href}>
          <h2>
            <Link href={t.href}>{t.title}</Link>
          </h2>
          <p>{t.blurb}</p>
          <p>
            <code>{t.api}</code> ·{" "}
            <Link href={t.example}>try an example</Link>
          </p>
        </section>
      ))}
    </main>
  );
}
