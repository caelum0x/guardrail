"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";

interface CompileFormProps {
  initialMandate?: string;
}

export function CompileForm({ initialMandate = "" }: CompileFormProps) {
  const router = useRouter();
  const [text, setText] = useState(initialMandate);

  function handleCompile() {
    const mandate = text.trim();
    if (!mandate) {
      return;
    }
    router.push(`/compile?mandate=${encodeURIComponent(mandate)}`);
  }

  return (
    <div className="actions" style={{ flexDirection: "column", alignItems: "stretch", gap: 12 }}>
      <textarea
        value={text}
        onChange={(event) => setText(event.target.value)}
        rows={4}
        placeholder="Trade CAKE, max drawdown 20%, kill switch 25%, stable reserve 10%"
        style={{ width: "100%", fontFamily: "inherit", padding: 8 }}
      />
      <div className="actions">
        <button type="button" className="buttonLink" onClick={handleCompile}>
          Compile
        </button>
      </div>
    </div>
  );
}
