# Strategy-Generation Prompt — Social / Sentiment Momentum

Use this prompt to generate a single decision payload for one rebalance cycle.

## Inputs you receive

A JSON object with:

- `fear_greed` — `{ value, value_classification }`
- `global` — `{ total_market_cap_usd, btc_dominance_pct }`
- `market_breadth` — `{ breadth_pct, median_24h_return }`
- `quotes[]` — per asset: `price_usd`, `percent_change_1h/24h/7d`,
  `market_cap_usd`, `volume_24h_usd`, `liquidity_usd`, `safety_score`,
  `security_flags[]`
- `attention[]` — per asset: `trending_rank`, `rank_prev`, `rank_delta`,
  `velocity_norm`, `volume_ratio`, `social_score`, `social_momentum`

## Procedure

1. **Classify the regime** with the `strategy_spec.yaml` rules. Record the
   `exposure_multiplier` (breakout 1.1, risk_on 1.0, chop 0.4, risk_off 0.15).
2. **Compute the sentiment gate**: `sentiment_gate_factor = 1.0` when
   `20 < fear_greed < 80`, else `0.6` (extreme-greed / extreme-fear de-risk).
3. **Score each non-stable asset**:
   - `trend_score`: rising trending rank / newly on the board → high; falling
     off the board → low.
   - `volume_component`: 1.0 when `volume_ratio >= 1.5` (confirmed by money);
     0.0 when `volume_ratio < 1.0` (hype without volume); interpolate between.
   - `social_component`: positive social momentum and `social_score >= 0.5` →
     high; flat/declining → low.
   - `attention_tilt = 0.4*trend_score + 0.35*volume_component + 0.25*social_component`.
   - `sentiment_score = clamp01(base_score adjusted by attention_tilt * sentiment_gate_factor) * (1 - security_penalty)`.
4. **Select entries**: assets with `sentiment_score >= 0.65`, capped at
   `max_positions` (5), ordered by `sentiment_score`. In chop, take only the 1-2
   cleanest, money-confirmed names and size down; in risk_off, take none
   (heartbeat only).
5. **Size**: `risk_budget = (100 - target_stable_reserve_pct) * exposure_multiplier`,
   clamped to `[0, 100 - min_stable_reserve]`. Allocate by `sentiment_score`
   share; cap each name at `max_position_weight_pct` (17%); surplus → USDT reserve.
6. **Emit actions**: `entry` / `rebalance` / `trim` / `exit` / `reject` /
   `heartbeat`, each with a `reason` citing the scores, and the resulting
   `target_portfolio` (weights summing to <= 100, USDT as the reserve remainder).

## Output

Return ONE JSON object shaped exactly like `examples/*.json`:
`{ scenario, as_of, inputs, computed: { market_regime, exposure_multiplier,
sentiment_gate_factor, sentiment_scores[], ... }, decision: { target_portfolio[],
rules: { entry[], exit[] }, actions[] } }`. Output JSON only — no prose around it.

## Guardrails

- Never exceed the 17% per-name cap or drop below the 10% minimum reserve.
- Never enter on `volume_ratio < 1.0`. Attention without participation is a trap.
- De-risk at sentiment extremes (gate factor 0.6 when fear_greed >= 80 or <= 20).
- The risk engine is the final authority; assume any over-allocation is clamped.
