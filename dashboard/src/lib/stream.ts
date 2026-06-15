import { API_URL } from "./api";

/**
 * A single event delivered over the SSE `/stream` endpoint. The API contract
 * guarantees at least these fields on each `data:` line; the payload shape is
 * event-specific so it stays an open record.
 */
export interface StreamEvent {
  event_type: string;
  timestamp: string;
  payload: Record<string, unknown>;
}

/** Connection states surfaced to subscribers so the UI can degrade gracefully. */
export type StreamStatus = "connecting" | "open" | "error";

export interface StreamHandlers {
  /** Called once per successfully parsed event. */
  onEvent: (event: StreamEvent) => void;
  /** Called whenever the underlying connection state changes. */
  onStatus?: (status: StreamStatus) => void;
}

/** Cleanup function returned by {@link subscribeToStream}. Always safe to call. */
export type StreamUnsubscribe = () => void;

function isBrowser(): boolean {
  return typeof window !== "undefined" && typeof window.EventSource !== "undefined";
}

/**
 * Narrows untrusted SSE JSON into a {@link StreamEvent}. Returns `null` when the
 * payload does not satisfy the contract so callers can skip malformed lines
 * instead of crashing the feed.
 */
function parseStreamEvent(raw: string): StreamEvent | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }

  if (typeof parsed !== "object" || parsed === null) {
    return null;
  }

  const record = parsed as Record<string, unknown>;
  const eventType = record.event_type;
  const timestamp = record.timestamp;
  if (typeof eventType !== "string" || typeof timestamp !== "string") {
    return null;
  }

  const payload =
    typeof record.payload === "object" && record.payload !== null
      ? (record.payload as Record<string, unknown>)
      : {};

  return { event_type: eventType, timestamp, payload };
}

/**
 * Opens an {@link EventSource} against `${API_URL}/stream`, parses each event,
 * and forwards it to the supplied handlers.
 *
 * SSR-safe: on the server (or any environment without `EventSource`) this is a
 * no-op that reports an `error` status and returns a cleanup function, so it can
 * be called unconditionally inside effects.
 */
export function subscribeToStream(handlers: StreamHandlers): StreamUnsubscribe {
  const { onEvent, onStatus } = handlers;

  if (!isBrowser()) {
    onStatus?.("error");
    return () => {};
  }

  onStatus?.("connecting");

  const source = new EventSource(`${API_URL}/stream`);

  const handleMessage = (message: MessageEvent<string>) => {
    const event = parseStreamEvent(message.data);
    if (event !== null) {
      onEvent(event);
    }
  };

  const handleOpen = () => onStatus?.("open");
  const handleError = () => onStatus?.("error");

  source.addEventListener("message", handleMessage);
  source.addEventListener("open", handleOpen);
  source.addEventListener("error", handleError);

  return () => {
    source.removeEventListener("message", handleMessage);
    source.removeEventListener("open", handleOpen);
    source.removeEventListener("error", handleError);
    source.close();
  };
}
