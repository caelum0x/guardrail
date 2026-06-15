# Strategy Generation Prompt

Generate a complete mean-reversion / range-fade strategy decision from the
supplied RSI / Bollinger / ATR + CMC data. Follow `strategy_spec.yaml` exactly.
Produce a JSON payload with the same shape as `examples/*.json`.

## Inputs you will receive

- `fear_greed`: `{ value (0..100), value_classification }`
- `global`: `{ total_market_cap_usd, btc_dominance_pct }`
- `market_breadth`: `{ breadth_pct, median_24h_return }`
- `indicators`: per asset — `{ symbol, rsi (0..100), percent_b, atr_pct }` from
  `crates/indicators` (RSI Wilder-14, Bollinger 20/2σ %B, ATR Wilder-14)
- `quotes`: per asset — `price_usd, percent_change_1h, percent_change_24h,
  market_cap_usd, volume_24h_usd, liquidity_usd, safety_score, security_flags`
- `asset_scores` (optional): base alpha score per asset; if absent, derive it the
  same way as the regime-routed-bsc-alpha Skill.

## Step-by-step procedure

### 1. Classify the regime (top-down, first match wins)
```
if breadth_pct >= 65 and median_24h_return > 2 and fear_greed >= 60  -> breakout
elif breadth_pct >= 55 and fear_greed >= 50                          -> risk_on
elif breadth_pct <= 40 or fear_greed <= 30 or median_24h_return < -2 -> risk_off
else                                                                  -> chop
```
Record the **inverted** `exposure_multiplier`: chop 1.0, risk_on 0.4,
breakout 0.2, risk_off 0.15. (Mean-reversion is most active in chop.)

### 2. Compute the reversion tilt per asset (in 0..1)
With RSI thresholds `oversold=30, overbought=70, breakdown=12, overbought_ref=50`
and %B thresholds `buy_zone_hi=0.20, mid=0.50, trim_zone_lo=0.80`:
```
# RSI component
if rsi <= breakdown:        rsi_score = 0.0                                  # broken, not fadeable
elif rsi >= overbought:     rsi_score = 0.0                                  # overbought, never initiate
elif breakdown < rsi <= oversold:  rsi_score = 1.0                          # prime oversold zone
else:                       rsi_score = clamp01((overbought_ref - rsi)/(overbought_ref - oversold))  # decays to mid

# Bollinger %B component
if percent_b <= buy_zone_hi: pb_score = 1.0                                  # at/below lower band
elif percent_b >= mid:       pb_score = 0.0                                  # no oversold dislocation
else:                        pb_score = clamp01((mid - percent_b)/(mid - buy_zone_hi))

reversion_tilt = 0.6 * rsi_score + 0.4 * pb_score                            # in [0, 1]
```

### 3. Tilt the base alpha score
```
factor          = 0.7 + 0.6 * reversion_tilt           # in [0.7, 1.3]
reversion_score = clamp01(base_score * factor * (1 - security_penalty))
```
Sort descending by `reversion_score`. Ties: lower RSI, then lower %B, then
liquidity, then symbol.

### 4. Build target weights (score-proportional allocator)
```
selected = [a for a in scored if a.reversion_score >= 0.65][:5]   # max_positions = 5
if not selected: target = 100% USDT
risk_budget = clamp((100 - 25) * exposure_multiplier, 0, 75)       # reserve = 25 (large)
for a in selected:
    w = risk_budget * (a.reversion_score / Σ selected.reversion_score)
    w = min(w, 17)                                                # per-name cap
target[USDT] = 100 - Σ allocated                                  # remainder (large by design)
```

### 5. Emit actions and risk view
- `entry` for each selected oversold asset; `trim`/`exit` for held assets that
  mean-reverted back (RSI through 55-60 / %B >= mid) or went overbought
  (RSI >= 70 / %B >= 0.80), fell below `min_score_to_hold` (0.50), broke down
  (RSI <= 12), or hit a stop/target; `no_entry`/`reject` with a reason for assets
  that did not qualify (e.g. overbought, broken-down, security-flagged).
- Add a `heartbeat` trade if no signal trade fired (daily-trade requirement).
- Include the effective `risk_policy` block and `daily_trade_requirement` status.

## Output

A single valid JSON object. Mirror the structure of `examples/chop_example.json`
(regime under `computed`, target portfolio + rules + actions under `decision`).
Do not invent fields. Keep weights summing to <= 100 (the remainder is the large
USDT reserve). Remember the strategy is heavy/active in `chop` and nearly
all-reserve in `breakout` / `risk_on` / `risk_off`.
