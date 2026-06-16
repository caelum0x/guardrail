# TA Studio (`/ta-studio`)

A server-rendered Next.js page that drives the read-only `GET /ta`
technical-analysis compute endpoint (`apps/guardrail-api/src/ta.rs`).

## What it does

- Calls `/ta` with a sensible default close-price series and `indicator=rsi`.
- Honours URL search params so the view is shareable/bookmarkable:
  `?indicator=sma|ema|rsi|macd|bollinger&series=1,2,3&period=14&mult=2`.
- Renders the returned indicator series in a per-step table. JSON `null`
  warmup positions are mapped to an em dash (`—`).
- Lists all available indicators with short descriptions, and offers one-click
  switches between them (preserving the current series/period/mult).

## Data flow

1. `searchParams` are parsed and narrowed (`parseIndicator`, `parsePeriod`,
   `parseMult`) with safe fallbacks.
2. A `URLSearchParams` query string is built and passed to
   `getJsonOrNull<TaResponse>` from `src/lib/api.ts`.
3. The response is type-guarded with `isTaSuccess` and normalised into table
   columns via `toColumns` — both in `src/lib/types-ta.ts`.

## Files

- `page.tsx` — the server component page.
- `../../lib/types-ta.ts` — response types, guards, and the column normaliser.

The nav link is registered in `src/components/Layout.tsx`.
