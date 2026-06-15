# Strategy-Generation Prompt — Momentum / Breakout

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
  `ema12`, `ema26`, `ema50`, `macd_hist`, `atr14`, `donchian_upper`,
  `breakout_above_upper`, `volume_ratio`

## Procedure

1. **Classify the regime** with the `strategy_spec.yaml` rules. Record the
   `exposure_multiplier` (breakout 1.1, risk_on 1.0, chop 0.4, risk_off 0.15).
2. **Score each non-stable asset**:
   - `trend_score`: aligned rising EMA stack (EMA12 > EMA26 > EMA50) and price
     above the stack → high; tangled/inverted → low.
   - `momentum_component`: positive and rising MACD histogram → high; negative →
     low (momentum rolling over).
   - `struct_score`: 1.0 when `close > donchian_upper` AND `volume_ratio >= 1.5`;
     0.0 when `volume_ratio < 1.0` (unconfirmed/fakeout); interpolate between.
   - `breakout_tilt = 0.4*trend_score + 0.35*momentum_component + 0.25*struct_score`.
   - `breakout_score = clamp01(base_score adjusted by breakout_tilt) * (1 - security_penalty)`.
3. **Select entries**: assets with `breakout_score >= 0.65`, capped at
   `max_positions` (5), ordered by `breakout_score`. In chop, take only the 1-2
   cleanest and size down; in risk_off, take none (heartbeat only).
4. **Size**: `risk_budget = (100 - target_stable_reserve_pct) * exposure_multiplier`,
   clamped to `[0, 100 - min_stable_reserve]`. Allocate by `breakout_score` share;
   cap each name at `max_position_weight_pct` (17%); surplus → USDT reserve.
5. **Emit actions**: `entry` / `rebalance` / `trim` / `exit` / `heartbeat`, each
   with a `reason` citing the scores, and the resulting `target_portfolio`
   (weights summing to <= 100, USDT as the reserve remainder).

## Output

Return ONE JSON object shaped exactly like `examples/*.json`:
`{ scenario, as_of, inputs, computed: { market_regime, exposure_multiplier,
breakout_scores[], ... }, decision: { target_portfolio[], rules: { entry[],
exit[] }, actions[] } }`. Output JSON only — no prose around it.

## Guardrails

- Never exceed the 17% per-name cap or drop below the 10% minimum reserve.
- Never enter on `volume_ratio < 1.0`. A breakout without participation is a trap.
- The risk engine is the final authority; assume any over-allocation is clamped.
