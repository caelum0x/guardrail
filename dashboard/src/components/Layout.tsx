import Link from "next/link";
import Image from "next/image";
import { AutoRefresh } from "./AutoRefresh";

const links = [
  "/",
  "/analytics",
  "/live",
  "/skills",
  "/portfolio",
  "/assets",
  "/watchlist",
  "/liquidity",
  "/indicators",
  "/ta-studio",
  "/fees",
  "/sizer",
  "/orderbook",
  "/pnl",
  "/correlation",
  "/quant",
  "/trending",
  "/quotes",
  "/costs",
  "/budget",
  "/optimizer",
  "/equity",
  "/exposure",
  "/trades",
  "/signals",
  "/heartbeat",
  "/rebalance",
  "/drift",
  "/exit-triggers",
  "/scenarios",
  "/regime",
  "/ensemble",
  "/journal",
  "/snapshots",
  "/funding",
  "/backtest",
  "/lab",
  "/walkforward",
  "/sweep",
  "/research",
  "/skill",
  "/bnb-sdk",
  "/agent-card",
  "/sdk-catalog",
  "/commerce",
  "/agent-services",
  "/job-simulator",
  "/signing-policy",
  "/experiments",
  "/compile",
  "/mandates",
  "/risk",
  "/alerts",
  "/playbook",
  "/briefing",
  "/prizes",
  "/scorecard",
  "/audit-manifest",
  "/readiness",
  "/events",
  "/observability",
  "/policy",
  "/wallet-controls",
  "/universe",
  "/config",
  "/ops",
  "/proof",
  "/compete",
  "/submission",
  "/reports",
  "/market-oracle",
  "/transports",
  "/journal-pro",
];

export function Layout({ children }: { children: React.ReactNode }) {
  return (
    <div>
      <AutoRefresh intervalMs={5000} />
      <nav className="nav">
        <Link href="/" className="brand" aria-label="Guardrail Alpha — home">
          <Image
            src="/logo.png"
            alt="Guardrail Alpha"
            width={124}
            height={68}
            priority
          />
        </Link>
        <div>
          {links.map((href) => (
            <Link key={href} href={href}>
              {href === "/" ? "Cockpit" : href.slice(1)}
            </Link>
          ))}
        </div>
      </nav>
      {children}
    </div>
  );
}
