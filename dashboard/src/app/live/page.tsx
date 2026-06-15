"use client";

import { useEffect, useState } from "react";
import { RegimeBadge } from "../../components/RegimeBadge";
import {
  subscribeToStream,
  type StreamEvent,
  type StreamStatus,
} from "../../lib/stream";

/** Maximum number of recent events retained in the rolling feed. */
const MAX_FEED_EVENTS = 50;

/** A received event tagged with a stable, monotonic ingestion id for React keys. */
interface KeyedEvent {
  id: number;
  event: StreamEvent;
}

interface LiveState {
  /** Most recently observed regime label, if any event carried one. */
  regime: string | null;
  /** Most recently observed NAV (USD) string, if any event carried one. */
  navUsd: string | null;
  /** Count of trade-shaped events seen this session. */
  tradeCount: number;
  /** Monotonic id assigned to the next received event. */
  nextId: number;
  /** Rolling list of recent events, newest first. */
  feed: KeyedEvent[];
}

const INITIAL_STATE: LiveState = {
  regime: null,
  navUsd: null,
  tradeCount: 0,
  nextId: 0,
  feed: [],
};

function isTradeEvent(eventType: string): boolean {
  const normalized = eventType.toLowerCase();
  return normalized.includes("trade") || normalized.includes("execution");
}

function readString(payload: Record<string, unknown>, key: string): string | null {
  const value = payload[key];
  if (typeof value === "string") {
    return value;
  }
  if (typeof value === "number" && Number.isFinite(value)) {
    return String(value);
  }
  return null;
}

/**
 * Folds a single incoming event into the accumulated live state without
 * mutating the previous state (immutable update).
 */
function reduceLive(state: LiveState, event: StreamEvent): LiveState {
  const regime = readString(event.payload, "regime") ?? state.regime;
  const navUsd =
    readString(event.payload, "nav_usd") ??
    readString(event.payload, "nav") ??
    state.navUsd;
  const tradeCount = isTradeEvent(event.event_type)
    ? state.tradeCount + 1
    : state.tradeCount;

  const keyed: KeyedEvent = { id: state.nextId, event };

  return {
    regime,
    navUsd,
    tradeCount,
    nextId: state.nextId + 1,
    feed: [keyed, ...state.feed].slice(0, MAX_FEED_EVENTS),
  };
}

function formatTimestamp(timestamp: string): string {
  const parsed = new Date(timestamp);
  if (Number.isNaN(parsed.getTime())) {
    return timestamp;
  }
  return parsed.toLocaleTimeString("en-US", { hour12: false });
}

/** Renders a single payload as a compact, human-scannable detail string. */
function compactDetail(payload: Record<string, unknown>): string {
  const entries = Object.entries(payload);
  if (entries.length === 0) {
    return "—";
  }
  return entries
    .slice(0, 4)
    .map(([key, value]) => {
      const rendered =
        typeof value === "object" && value !== null
          ? JSON.stringify(value)
          : String(value);
      const trimmed = rendered.length > 48 ? `${rendered.slice(0, 45)}…` : rendered;
      return `${key}=${trimmed}`;
    })
    .join("  ");
}

function statusText(status: StreamStatus): string {
  if (status === "open") {
    return "Live";
  }
  if (status === "connecting") {
    return "Connecting…";
  }
  return "Waiting for /stream…";
}

function statusBadgeClass(status: StreamStatus): string {
  if (status === "open") {
    return "badge";
  }
  if (status === "connecting") {
    return "badge badgeWarning";
  }
  return "badge badgeCritical";
}

export default function LivePage() {
  const [state, setState] = useState<LiveState>(INITIAL_STATE);
  const [status, setStatus] = useState<StreamStatus>("connecting");

  useEffect(() => {
    const unsubscribe = subscribeToStream({
      onEvent: (event) => setState((prev) => reduceLive(prev, event)),
      onStatus: (next) => setStatus(next),
    });
    return unsubscribe;
  }, []);

  const regimeLabel = state.regime ?? (status === "open" ? "Awaiting regime" : "—");

  return (
    <main className="grid">
      <section className="hero">
        <div>
          <p className="eyebrow">Real-time stream</p>
          <h1>Live Feed</h1>
        </div>
        <span className={statusBadgeClass(status)}>{statusText(status)}</span>
      </section>

      <section className="panel wide">
        <div className="hero">
          <div>
            <p className="eyebrow">Latest regime</p>
          </div>
          <RegimeBadge regime={regimeLabel} />
        </div>
        <div className="metricGrid">
          <div>
            <span>NAV (USD)</span>
            <strong>{state.navUsd !== null ? `$${state.navUsd}` : "—"}</strong>
          </div>
          <div>
            <span>Trades seen</span>
            <strong>{state.tradeCount}</strong>
          </div>
          <div>
            <span>Events buffered</span>
            <strong>{state.feed.length}</strong>
          </div>
          <div>
            <span>Connection</span>
            <strong>{statusText(status)}</strong>
          </div>
        </div>
      </section>

      <section className="panel wide">
        <p className="eyebrow">Recent events</p>
        {state.feed.length === 0 ? (
          <p className="mono">
            {status === "error"
              ? "Waiting for /stream… (stream unavailable)"
              : "Waiting for /stream…"}
          </p>
        ) : (
          <ul className="streamFeed">
            {state.feed.map(({ id, event }) => (
              <li key={id} className="streamRow">
                <span className="badge">{event.event_type}</span>
                <span className="mono">{formatTimestamp(event.timestamp)}</span>
                <span className="mono streamDetail">{compactDetail(event.payload)}</span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </main>
  );
}
