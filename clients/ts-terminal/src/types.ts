// Typed views of the Guardrail API responses this CLI reads. Fields are
// optional because the API is offline-safe and some are absent in paper mode.

export type Json = Record<string, unknown>;

export interface Health {
  status?: string;
  events?: number;
}

export interface Regime {
  regime?: string;
  exposure?: string | number;
}

export interface VerifyCheck {
  name: string;
  status: string;
  detail: string;
}

export interface Verify {
  passed?: boolean;
  reason?: string;
  checks?: VerifyCheck[];
}

export interface EventsResponse {
  events?: Array<{
    timestamp?: string;
    event_type?: string;
    payload_json?: Json;
  }>;
}
