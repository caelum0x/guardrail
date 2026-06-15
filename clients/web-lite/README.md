# Guardrail Web-Lite — Mission Control

A **zero-dependency, single-file** mission-control cockpit for the Guardrail agent. It is one
self-contained `index.html` (inline CSS + inline ES module, no build step, no external CDNs)
that calls the Guardrail API directly from the browser and renders a live operator view.

It is **complementary to the Next.js `dashboard/`**: where the dashboard is a full app, web-lite
is a drop-anywhere, embeddable status page you can open from a file, paste into a static host,
or serve same-origin alongside the API.

## Tabs

The cockpit is a multi-tab single-page app. The top tab bar switches views without a reload and
is **deep-linkable + persisted** via the URL hash (and `localStorage` as a fallback):

| Tab | Hash | Source |
| --- | --- | --- |
| **Overview** | `#overview` | `/compete`, `/history`, `/alerts`, `/regime`, `/proof` |
| **Backtest** | `#backtest` | `/backtest?preset=…` |
| **Indicators** | `#indicators` | `/indicators?symbol=…` (symbols sourced from `/assets`) |
| **Trending** | `#trending` | `/trending` |
| **Assets** | `#assets` | `/assets` |
| **Walk-forward** | `#walkforward` | `/walkforward?preset=…` |
| **Optimize** | `#optimize` | `/optimize` |
| **Experiments** | `#experiments` | `/experiments` |
| **Skill** | `#skill` | `/skill` |
| **Funding** | `#funding` | `/funding` |
| **Scenarios** | `#scenarios` | `/scenarios` |
| **Costs** | `#costs` | `/costs` |
| **Liquidity** | `#liquidity` | `/liquidity` |
| **Readiness** | `#readiness` | `/readiness` |
| **Ensemble** | `#ensemble` | `/regime` (+ embedded regime→skill weight table) |
| **Journal** | `#journal` | `/events` |
| **Signing** | `#signing` | `/signing-policy` |

Refreshing every 5 seconds, each panel is fetched independently and degrades to a placeholder if
the API is unreachable (no fetch ever throws uncaught).

### Overview

- **`/compete`** — readiness chips: registered, eligible assets, daily-trade, confirmed trades, kill switch
- **`/history`** — inline SVG NAV equity sparkline + latest NAV + % change since start
- **`/alerts`** — alert count + severity badge (clear / warning / critical)
- **`/regime`** — current market regime + exposure multiplier (+ regime inputs)
- **`/proof`** — agent id / wallet / policy hash / registration tx

### Backtest

- **`/backtest`** — preset selector (conservative / balanced / aggressive), an inline SVG equity
  curve (strategy line with a dashed buy-and-hold benchmark baseline), and a metrics grid:
  total return, max drawdown, Calmar ratio, volatility, win rate, profit factor, trade count,
  benchmark return, excess return, final NAV.

### Indicators

- **`/indicators`** — symbol selector populated from the eligible universe (`/assets`), showing the
  latest EMA, SMA, RSI, MACD (macd / signal / histogram), Bollinger Bands (upper / mid / lower),
  and ATR-style close stats. RSI is color-coded for overbought (≥70) / oversold (≤30).

### Trending

- **`/trending`** — a ranked table of trending tokens (rank, symbol, CMC id).

### Assets

- **`/assets`** — the eligible universe rendered as a grid of cards (symbol, category, price,
  24h change).

### Walk-forward

- **`/walkforward`** — preset selector (conservative / balanced / aggressive), an inline SVG bar
  strip of per-window out-of-sample returns (green for positive, red for negative around a zero
  baseline), and a per-window table: window index, fear/greed reading, in-sample (benchmark)
  return, out-of-sample (strategy) return, drawdown, trades, and a pass/fail badge derived from
  excess return. A final aggregate row summarizes mean excess, worst drawdown, and the count of
  positive windows.

### Optimize

- **`/optimize`** — the portfolio optimizer's allocation methods (equal weight, score
  proportional, inverse volatility, risk parity) rendered as a ranked table: per-symbol weight
  columns plus a top-weight (concentration) score, ranked descending with the top row
  highlighted.

### Experiments

- **`/experiments`** — registered experiments / A-B variants rendered as cards. Each card adapts
  to the payload: name/tag, a status badge, a key metric (objective / score / return / Sharpe, or
  any numeric field), creation time, and preset where present.

### Skill

- **`/skill`** — the active Track-2 CMC Skill (`cmc-regime-routed-alpha`). The API returns the
  skill artifact text verbatim (no server-side YAML parser), so the cockpit renders: a summary
  card (name, version/author and a description blurb parsed from `skill.yaml` / `README.md`), the
  list of example filenames, and a **regime routing** table parsed from any `regime → target
  weights` lines found in the artifact (degrades to a note when no structured mapping is present).

### Funding

- **`/funding`** — a funding-rate table over the non-stable eligible universe: symbol, the
  synthetic per-hour `funding_rate_proxy`, 24h return, and a signal/tilt badge. Color-coded by
  sign — negative funding is favourable (green, long tilt), positive funding is crowded (red,
  short tilt), near-zero is neutral.

### Scenarios

- **`/scenarios`** — the scenario stress desk: a summary (portfolio NAV, worst scenario id, worst
  P&L) plus a table of category-shock scenarios with portfolio P&L, return, the largest single
  loss, and a status badge (normal / watch / critical).

### Costs

- **`/costs`** — the execution cost preview: a metrics grid (order notional, routes priced, total
  gas, total slippage, all-in cost, average cost in bps) plus a per-route breakdown table (route,
  side, notional, gas, slippage, all-in cost, cost in bps, price impact). Read-only — no quotes or
  swaps are submitted.

### Liquidity

- **`/liquidity`** — per-asset liquidity capacity over the non-stable eligible universe: a summary
  (assets, ok / watch / blocking counts) plus a table of symbol, liquidity (abbreviated USD), pool
  usage %, capacity, headroom, 24h return, and an adequacy status badge color-coded by the API's
  `ok` / `watch` / `blocking` classification.

### Readiness

- **`/readiness`** — the judge-facing competition readiness checklist: a headline pass count and an
  overall ready / blocking badge, plus a table where each check (run report, wallet, policy hash,
  event log, market data, risk decisions, TWAK quotes, confirmed txs, daily trade, critical
  alerts) renders its detail and a pass/fail badge.

### Ensemble

- **`/regime`** — the **ensemble router**. Fetches the current market regime, then renders the four
  Track-2 candidate skills (`regime-routed-bsc-alpha`, `funding-rate-carry-bsc`,
  `mean-reversion-chop-bsc`, `trend-breakout-momentum-bsc`) with each skill's weight in the
  **current** regime and a one-line thesis. The dominant skill(s) for the live regime are
  highlighted. The regime→weight table is embedded as a JS constant (`ENSEMBLE_WEIGHTS`) that
  mirrors `skills/ensemble.json` — each regime (`risk_on` / `risk_off` / `chop` / `breakout`) sums
  to 1.0.

### Journal

- **`/events`** — a reverse-chronological, human-readable **decision journal**. Each agent event
  (regime classifications, asset scores, target/risk decisions, quotes, swaps, confirmed txs,
  reconciliations, kill switch, reports) renders as a row with a localized timestamp, a type badge
  color-coded by severity, and a compact human-readable detail summarizing the payload. Guards and
  shows a placeholder when the API is unreachable or there are no entries.

### Signing

- **`/signing-policy`** — the judge-facing **self-custody signing policy** panel ("keys never leave
  the user"). Renders a labeled grid (status, chain id, allowed/denied action counts, priced
  resources) plus a key/value list of limits and custody facts (who holds keys, payment/accepts
  headers, payment token, per-call and session budgets, validity windows, sample payer and
  authorization hash) and chips for the allowed (green) / denied (red) EIP-712 primary types.

## How to open

Option A — just open the file:

```bash
open clients/web-lite/index.html        # macOS
# or double-click index.html in a file browser
```

Option B — serve it locally (handy for clean URLs and to mirror a hosted setup):

```bash
cd clients/web-lite
python3 -m http.server 5500
# then visit http://localhost:5500
```

## API base URL configuration

The page resolves the API base in this order:

1. **`?api=` query param** — e.g. `index.html?api=http://localhost:8080`
   (this also gets persisted to `localStorage`)
2. **`localStorage`** — whatever you last typed into the **API base** field in the header
3. **Default** — `http://localhost:8080`

You can change it at any time via the **API base** input in the top-right; the value persists
across reloads.

## CORS note

Because this page runs **in the browser**, when it is served from a *different* origin than the
API, the Guardrail API (built on `tower-http`) must send permissive CORS headers. Add a
`tower_http::cors::CorsLayer` to the API router so cross-origin `GET`s are allowed.

If you instead serve `index.html` from the **same origin** as the API (same host and port, e.g.
behind a reverse proxy or mounted as a static route), **no CORS configuration is needed**.
