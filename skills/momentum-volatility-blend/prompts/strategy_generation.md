# Strategy-Generation Prompt — Momentum + Volatility-Quality Blend

Use this prompt to generate a single decision payload for one rebalance cycle.

## Inputs you receive

A JSON object with:

- `fear_greed` — `{ value, value_classification }`
- `global` — `{ total_market_cap_usd, btc_dominance_pct }`
- `market_breadth` — `{ breadth_pct, median_24h_return }`
- `quotes[]` — per asset: `price_usd`, `percent_change_1h/24h/7d`,
  `market_cap_usd`, `volume_24h_usd`, `liquidity_usd`, `volatility_1h`,
  `safety_score`, `security_flags[]`
- `indicators[]` — per asset, derived from OHLCV via `crates/indicators`:
  `ema12`, `ema26`, `ema50`, `macd_hist`, `atr14`, `realised_vol_annual_pct`

## Procedure

1. **Classify the regime** with the `strategy_spec.yaml` rules. Record the
   `exposure_multiplier` (breakout 1.1, risk_on 1.0, chop 0.4, risk_off 0.15).
2. **Score each non-stable asset**:
   - `trend_score`: aligned rising EMA stack (EMA12 > EMA26 > EMA50) with price
     above → 1.0; price > EMA26 but tangled → 0.5; downtrend → 0.0.
   - `momentum_score`: `clamp01(0.6*clamp01(pct_change_7d/20) + 0.4*clamp01(macd_hist/atr14))`;
     0.0 when `macd_hist <= 0`.
   - `momentum_leg = 0.5*trend_score + 0.5*momentum_score`; forced 0 in a
     downtrend or on a negative MACD histogram.
   - `vol_quality`: bell over `realised_vol_annual_pct` — 1.0 in 50-80, linear
     ramp 40→50 and 80→110, 0.0 below 40 (dead) or above 110 (blow-off).
   - `blend_tilt = momentum_leg * vol_quality`.
   - `blend_score = clamp01(base_score adjusted by blend_tilt) * (1 - security_penalty)`.
3. **Select entries**: assets with `blend_score >= 0.65`, capped at
   `max_positions` (5), ordered by `blend_score`. In chop, take only the 1-2
   highest-quality and size down; in risk_off, take none (heartbeat only).
4. **Size**: `risk_budget = (100 - target_stable_reserve_pct) * exposure_multiplier`,
   clamped to `[0, 100 - min_stable_reserve]`. Allocate by `blend_score` share;
   cap each name at `max_position_weight_pct` (17%); surplus → USDT reserve.
5. **Emit actions**: `entry` / `rebalance` / `trim` / `exit` / `heartbeat`, each
   with a `reason` citing the scores (especially `vol_quality` for any demotion),
   and the resulting `target_portfolio` (weights summing to <= 100, USDT as the
   reserve remainder).

## Output

Return ONE JSON object shaped exactly like `examples/*.json`:
`{ scenario, as_of, inputs, computed: { market_regime, exposure_multiplier,
blend_scores[], ... }, decision: { target_portfolio[], rules: { entry[],
exit[] }, actions[] } }`. Output JSON only — no prose around it.

## Guardrails

- Never exceed the 17% per-name cap or drop below the 10% minimum reserve.
- Never buy a strong trend at DEAD vol (< 40%) or BLOW-OFF vol (> 110%) —
  `vol_quality` is 0, so `blend_score` collapses regardless of momentum.
- Never enter in a downtrend or on a negative MACD histogram.
- The risk engine is the final authority; assume any over-allocation is clamped.
