"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";

interface LabControlsProps {
  steps: number;
  fearGreed: number;
}

export function LabControls({ steps, fearGreed }: LabControlsProps) {
  const router = useRouter();
  const [stepsValue, setStepsValue] = useState<number>(steps);
  const [fearGreedValue, setFearGreedValue] = useState<number>(fearGreed);

  function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    router.push(`/lab?steps=${stepsValue}&fear_greed=${fearGreedValue}`);
  }

  return (
    <form className="actions" onSubmit={handleSubmit}>
      <label>
        <span>Steps</span>
        <input
          type="number"
          min={2}
          max={1000}
          value={stepsValue}
          onChange={(e) => setStepsValue(Number(e.target.value))}
        />
      </label>
      <label>
        <span>Fear &amp; Greed</span>
        <input
          type="range"
          min={0}
          max={100}
          value={fearGreedValue}
          onChange={(e) => setFearGreedValue(Number(e.target.value))}
        />
        <input
          type="number"
          min={0}
          max={100}
          value={fearGreedValue}
          onChange={(e) => setFearGreedValue(Number(e.target.value))}
        />
      </label>
      <button type="submit" className="buttonLink">
        Run
      </button>
    </form>
  );
}
