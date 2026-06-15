# Strategy Generation Prompt — <PLACEHOLDER_SKILL_NAME>  (TEMPLATE)

Generate a complete strategy decision from the supplied CMC + signal data. Follow
`strategy_spec.yaml` exactly. Produce a JSON payload with the same shape as
`examples/*.json`.

## Inputs you will receive

- `fear_greed`: `{ value (0..100), value_classification }`
- `global`: `{ total_market_cap_usd, btc_dominance_pct }`
- `market_breadth`: `{ breadth_pct, median_24h_return }`
- `quotes`: per asset — `price_usd, percent_change_1h, percent_change_24h,
  market_cap_usd, volume_24h_usd, liquidity_usd, volatility_1h, safety_score,
  security_flags`
- `asset_scores` (optional): base alpha score per asset; if absent, derive it the
  same way as the shared alpha blend.
- `<PLACEHOLDER signal feed>`: per asset — `{ symbol, <raw_signal field> }`.

## Step-by-step procedure

### 1. Classify the regime (top-down, first match wins)
```
if breadth_pct >= 65 and median_24h_return > 2 and fear_greed >= 60  -> breakout
elif breadth_pct >= 55 and fear_greed >= 50                          -> risk_on
elif breadth_pct <= 40 or fear_greed <= 30 or median_24h_return < -2 -> risk_off
else                                                                  -> chop
```
Record `exposure_multiplier`: breakout 1.1, risk_on 1.0, chop 0.5, risk_off 0.2.

### 2. Compute the signal tilt per asset (in 0..1)
```
<PLACEHOLDER> map the raw signal to a tilt in [0, 1].
tilt = clamp01( <function of raw_signal and the thresholds in strategy_spec.yaml> )
```

### 3. Tilt the base alpha score
```
factor = 0.7 + 0.6 * tilt                       # in [0.7, 1.3]
score  = clamp01(base_score * factor * (1 - security_penalty))
```
Sort descending by `score`. Ties: higher tilt, then liquidity, then symbol.

### 4. Build target weights (score-proportional allocator)
```
selected = [a for a in scored if a.score >= 0.65][:5]   # max_positions = 5
if not selected: target = 100% USDT
risk_budget = clamp((100 - 15) * exposure_multiplier, 0, 85)   # reserve = 15
for a in selected:
    w = risk_budget * (a.score / Σ selected.score)
    w = min(w, 17)                                       # per-name cap
target[USDT] = 100 - Σ allocated                         # remainder
```

### 5. Emit actions and risk view
- `entry` for each selected asset, `exit` for held assets that fell below
  `min_score_to_hold` (0.50) or on a regime downgrade; `no_entry`/`reject` with a
  reason for assets that did not qualify.
- Add a `heartbeat` trade if no signal trade fired (daily-trade requirement).
- Include the effective `risk_policy` block and `daily_trade_requirement` status.

## Output

A single valid JSON object. Mirror the structure of `examples/risk_on_example.json`
(regime under `computed`, target portfolio + rules + actions under `decision`).
Do not invent fields. Keep weights summing to <= 100 (the remainder is the USDT
reserve).
