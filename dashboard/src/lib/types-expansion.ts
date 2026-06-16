// Types for the expansion pages (/market-oracle, /transports, /journal-pro).
// Kept separate from the existing types.ts so the additions are self-contained.

export interface CmcDataset {
  dataset: string;
  cmc: string;
  source: string;
  powers: string[];
}

export interface CmcCapability {
  capability: string;
  description: string;
  cmc_inputs: string[];
  api?: string;
  mcp_tool?: string;
}

export interface CmcCapabilitiesResponse {
  status?: string;
  source?: string;
  summary?: {
    cmc_datasets?: number;
    exposed_capabilities?: number;
    execution_exposed?: boolean;
  };
  descriptor?: {
    agent?: string;
    summary?: string;
    execution_policy?: string;
    datasets?: CmcDataset[];
    capabilities?: CmcCapability[];
  };
}

// A stored event as returned by GET /events ({ events: StoredEvent[] }).
export interface StoredEvent {
  id: string;
  run_id: string;
  timestamp: string;
  event_type: string;
  payload_json: Record<string, unknown>;
}

export interface EventsResponse {
  events?: StoredEvent[];
  error?: string;
}
