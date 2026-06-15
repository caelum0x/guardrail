import { SignalTable } from "../../components/SignalTable";
import { getJsonOrNull } from "../../lib/api";
import type { SignalsResponse } from "../../lib/types";

export default async function SignalsPage() {
  const signals = await getJsonOrNull<SignalsResponse>("/signals");
  return (
    <main className="grid">
      <SignalTable signals={signals} />
    </main>
  );
}
