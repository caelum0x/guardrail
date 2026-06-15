"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

interface AutoRefreshProps {
  /** Refresh interval in milliseconds. */
  intervalMs?: number;
}

/**
 * Periodically re-runs the current route's server components so the dashboard
 * reflects the latest agent state without a manual reload. Renders nothing.
 */
export function AutoRefresh({ intervalMs = 5000 }: AutoRefreshProps) {
  const router = useRouter();

  useEffect(() => {
    const id = setInterval(() => router.refresh(), intervalMs);
    return () => clearInterval(id);
  }, [router, intervalMs]);

  return null;
}
