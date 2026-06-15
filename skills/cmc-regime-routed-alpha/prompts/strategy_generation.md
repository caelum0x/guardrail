# Strategy Generation Prompt

Generate a complete regime-routed strategy decision from the supplied CMC data.
Follow `strategy_spec.yaml` exactly. Produce a JSON payload with the same shape
as `examples/*.json`.

## Inputs you will receive

- `fear_greed`: `{ value (0..100), value_classification }`
- `global`: `{ total_market_cap_usd, btc_dominance_pct }`
- `market_breadth`: `{ breadth_pct, median_24h_return }`
- `trending`: list of symbols currently trending on CMC
- `quotes`: per asset — `price_usd, percent_change_1h, percent_change_24h,
  market_cap_usd, volume_24h_usd, liquidity_usd, volatility_1h, safety_score,
  security_flags`
- (optional) `ohlcv`: candle history for RSI(14) / MACD(12,26,9) confirmation

## Step-by-step procedure

### 1. Classify the regime (top-down, first match wins)
```
if breadth_pct >= 65 and median_24h_return > 2 and fear_greed >= 60  -> breakout
elif breadth_pct >= 55 and fear_greed >= 50                          -> risk_on
elif breadth_pct <= 40 or fear_greed <= 30 or median_24h_return < -2 -> risk_off
else                                                                  -> chop
```
Record `exposure_multiplier`: breakout 1.1, risk_on 1.0, chop 0.5, risk_off 0.2.

### 2. Compute per-asset features (each normalized to 0..1)
- `momentum   = sigmoid((0.6*ret_1h + 0.4*ret_24h) / 5)` — confirm with RSI(14)
  in 40..70 and MACD histogram > 0 and rising.
- `volume     = min_max(volume_24h / market_cap, 0, 0.5)`
- `volatility = clamp01(1 - |volatility_1h - 3| / 6)`
- `liquidity  = min_max(log10(liquidity_usd), 5, 8)`
- `sentiment  = fg<=75 ? clamp01(fg/75) : clamp01(1 - (fg-75)/50)`
- `execution_quality = clamp01(1 - (2000 / liquidity_usd) * 20)`
- `security_penalty   = clamp01((100 - safety_score)/100 * 0.5 + 0.25 * num_flags)`

### 3. Blend into the alpha score
```
weights = { momentum:0.30, volume:0.15, volatility:0.10,
            liquidity:0.15, sentiment:0.10, execution_quality:0.20 }
raw   = Σ weight_i * feature_i
norm  = raw / Σ weight_i
score = clamp01(norm * (1 - security_penalty))
```
Sort descending by score.

### 4. Build target weights (score-proportional allocator)
```
selected = [a for a in scored if a.score >= 0.65][:5]   # max_positions = 5
if not selected: target = 100% USDT
risk_budget = clamp((100 - 15) * exposure_multiplier, 0, 85)   # reserve = 15
for a in selected:
    w = risk_budget * (a.score / Σ selected.score)
    w = min(w, 17)                                             # per-name cap
target[USDT] = 100 - Σ allocated                               # remainder
```

### 5. Emit actions and risk view
- `entry` for each selected asset, `exit` for held assets that fell below
  `min_score_to_hold` (0.50) or on a regime downgrade, `no_entry`/`reject` with a
  reason for assets that did not qualify.
- Add a `heartbeat` trade if no signal trade fired (daily-trade requirement).
- Include the effective `risk_policy` block and `daily_trade_requirement` status.

## Output

A single valid JSON object. Mirror the structure of `examples/risk_on_example.json`.
Do not invent fields. Keep weights summing to ~100.
