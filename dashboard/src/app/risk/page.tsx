import { RiskPanel } from "../../components/RiskPanel";
import { getJsonOrNull } from "../../lib/api";
import type { RiskResponse } from "../../lib/types";

export default async function RiskPage() {
  const risk = await getJsonOrNull<RiskResponse>("/risk");
  return (
    <main className="grid">
      <RiskPanel risk={risk} />
    </main>
  );
}
