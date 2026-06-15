"use client";

import { useRouter, usePathname, useSearchParams } from "next/navigation";

interface PresetSelectProps {
  current: string;
}

const PRESET_OPTIONS = ["conservative", "balanced", "aggressive"] as const;

/**
 * Strategy-preset selector. Renders a <select> defaulting to the current
 * preset, and on change navigates to the same pathname with all existing
 * query params preserved (steps/fear_greed/etc.) and only `preset` updated.
 */
export function PresetSelect({ current }: PresetSelectProps) {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();

  function handleChange(event: React.ChangeEvent<HTMLSelectElement>) {
    const params = new URLSearchParams(searchParams.toString());
    params.set("preset", event.target.value);
    router.push(`${pathname}?${params.toString()}`);
  }

  return (
    <label>
      <span>Strategy preset</span>
      <select value={current} onChange={handleChange}>
        {PRESET_OPTIONS.map((option) => (
          <option key={option} value={option}>
            {option}
          </option>
        ))}
      </select>
    </label>
  );
}
