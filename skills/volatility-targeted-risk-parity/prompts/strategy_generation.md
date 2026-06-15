# Strategy-Generation Prompt — Volatility-Targeted Risk Parity

Use this prompt to generate a single decision payload for one rebalance cycle.

## Inputs you receive

A JSON object with:

- `fear_greed` — `{ value, value_classification }`
- `global` — `{ total_market_cap_usd, btc_dominance_pct }`
- `market_breadth` — `{ breadth_pct, median_24h_return }`
- `quotes[]` — per asset: `price_usd`, `percent_change_1h/24h/7d`,
  `market_cap_usd`, `volume_24h_usd`, `liquidity_usd`, `safety_score`,
  `security_flags[]`
- `indicators[]` — per asset, derived from OHLCV via `crates/indicators`:
  `realised_vol` (trailing annualised realised volatility, fraction),
  `atr14`, `atr_pct`

## Procedure

1. **Classify the regime** with the `strategy_spec.yaml` rules. Record the regime
   `exposure_multiplier` (breakout 1.0, risk_on 1.0, chop 0.8, risk_off 0.4).
2. **Build the eligible set**: non-stable, enabled, liquid, security-clean names
   with `realised_vol > 0`. Drop any name with no usable vol.
3. **Inverse-vol weights** (the risk-parity core, mirroring
   `crates/portfolio-optimizer::inverse_volatility` / `risk_parity_lite`):
   - `inv_vol_i = 1 / max(realised_vol_i, vol_floor=0.05)`.
   - `raw_parity_weight_i = inv_vol_i / Σ_j inv_vol_j` (sums to 1).
   - Apply the security haircut `raw_parity_weight_i *= (1 - security_penalty_i)`
     and re-normalise. Each name now contributes ~equal risk (`weight * vol`).
4. **Target-vol scalar**: `est_book_vol = Σ_i raw_parity_weight_i * realised_vol_i`;
   `target_vol_scalar = clamp(0.45 / est_book_vol, 0.20, 1.00)`. High vol => < 1
   (de-risk); calm => -> 1.0 (never > 1.0, no leverage).
5. **Size**: `gross = 100 * exposure_multiplier * target_vol_scalar`, clamped to
   `[0, 100 - min_stable_reserve]`. `weight_i = gross * raw_parity_weight_i`; cap
   each at `max_position_weight_pct` (17%); surplus over the cap and the un-deployed
   gross fall to the USDT reserve. Keep at most `max_sleeves` (12) names.
6. **Emit actions**: `entry` / `rebalance` / `trim` / `exit` / `heartbeat`, each
   with a `reason` citing the vol and the parity weight, and the resulting
   `target_portfolio` (weights summing to <= 100, USDT as the reserve remainder).

## Output

Return ONE JSON object shaped exactly like `examples/*.json`:
`{ scenario, as_of, inputs, computed: { market_regime, exposure_multiplier,
target_vol, gross_pct, risk_weights[], ... }, decision: { target_portfolio[],
rules: { entry[], exit[] }, actions[] } }`. Output JSON only — no prose around it.

## Guardrails

- Never exceed the 17% per-name cap or drop below the 10% minimum reserve.
- Never lever: `target_vol_scalar <= 1.0` always (spot-only book).
- Size by `1 / realised_vol`, never by a directional score. No view, only risk.
- The risk engine is the final authority; assume any over-allocation is clamped.
