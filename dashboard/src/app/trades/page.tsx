import { TradeTimeline } from "../../components/TradeTimeline";
import { getJsonOrNull } from "../../lib/api";
import type { TradesResponse } from "../../lib/types";

export default async function TradesPage() {
  const trades = await getJsonOrNull<TradesResponse>("/trades");
  return (
    <main className="grid">
      <TradeTimeline events={trades?.trades ?? []} />
    </main>
  );
}
