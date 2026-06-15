import { PortfolioTable } from "../../components/PortfolioTable";
import { getJsonOrNull } from "../../lib/api";
import type { PortfolioResponse } from "../../lib/types";

export default async function PortfolioPage() {
  const portfolio = await getJsonOrNull<PortfolioResponse>("/portfolio");
  return (
    <main className="grid">
      <PortfolioTable portfolio={portfolio} />
    </main>
  );
}
