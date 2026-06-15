import Link from "next/link";
import { AutoRefresh } from "./AutoRefresh";

const links = [
  "/",
  "/live",
  "/skills",
  "/portfolio",
  "/assets",
  "/watchlist",
  "/liquidity",
  "/indicators",
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
  "/reports",
];

export function Layout({ children }: { children: React.ReactNode }) {
  return (
    <div>
      <AutoRefresh intervalMs={5000} />
      <nav className="nav">
        <strong>Guardrail Alpha</strong>
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
