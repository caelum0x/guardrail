# Strategy Comparison — regime-routed-bsc-alpha vs funding-rate-carry-bsc

Both skills are Track-2 strategy specifications over the **same 20 eligible BSC
tokens**, sharing the **same regime model**, **same score-proportional
allocator**, and **same Rust risk envelope**. They differ in *how a candidate
asset earns its rank*. `funding-rate-carry-bsc` is an explicit sibling of
`regime-routed-bsc-alpha`: it reuses the latter's base alpha score and layers a
funding-carry tilt on top.

## Side-by-side

| Dimension | regime-routed-bsc-alpha | funding-rate-carry-bsc |
|-----------|-------------------------|------------------------|
| **Core thesis** | Rank assets by a broad multi-factor alpha blend, route exposure by regime. | Same base alpha, but tilt toward assets with favourable perp funding (long basis carry) and fade crowded longs. |
| **Signal source** | `alpha_score` = momentum (RSI/MACD-confirmed) + volume + volatility + liquidity + sentiment + execution-quality, haircut by a security penalty. | `carry_score = clamp01(base_alpha * (0.7 + 0.6 * funding_tilt) * (1 - security_penalty))` — the base alpha re-weighted by a funding tilt. |
| **Distinguishing input** | `cmc_ohlcv` (drives RSI(14), MACD(12,26,9), ATR volatility) and `cmc_trending` (soft confirmation). | `funding_rate_proxy` (per-hour synthetic funding) and `asset_scores` (the base alpha reused from the alpha skill). |
| **Data inputs** | `cmc_quotes`, `cmc_ohlcv`, `cmc_fear_greed`, `cmc_dex_liquidity`, `cmc_token_security`, `cmc_trending`, `cmc_global`, `eligible_asset_list`. | `cmc_quotes`, `funding_rate_proxy`, `cmc_fear_greed`, `cmc_dex_liquidity`, `cmc_token_security`, `cmc_global`, `asset_scores`, `eligible_asset_list`. (No OHLCV/RSI/MACD or trending of its own — it inherits the base score.) |
| **Ranking output** | `asset_scores`: `{ symbol, score, components, security_penalty }`. | `carry_scores`: `{ symbol, base_score, funding_rate_proxy, funding_tilt, carry_score, security_penalty }`. |
| **Regime routing** | Identical: top-down `breakout / risk_on / risk_off / chop` with exposure multipliers 1.1 / 1.0 / 0.5 / 0.2. | Identical regime model and multipliers; `risk_off` rotates to reserve **regardless of how attractive funding looks**. |
| **Allocator** | Score-proportional, per-name cap 17%, 15% target USDT reserve, max 5 positions, min score 0.65. | Same allocator and caps, but weights are proportional to `carry_score` instead of raw alpha. |
| **Extra entry/exit logic** | RSI/MACD confirmation; trending as a soft tiebreaker. | No-initiate if funding `>= 0.30` (crowded long) or `<= -0.40` (capitulation); trim/exit a held name if funding crosses those edges. |
| **Spec version** | 2.0.0 | 1.0.0 |

## Regime routing behaviour — identical envelope, different inputs to the rank

Both skills classify the market the same way and apply the same exposure
multipliers (breakout 1.1, risk_on 1.0, chop 0.5, risk_off 0.2). The regime
decides **how much** risk budget is deployed; the per-asset score decides **which
names** fill it. The two skills only diverge on the second step:

- **regime-routed-bsc-alpha** lets the multi-factor alpha blend pick the names.
- **funding-rate-carry-bsc** takes that same blend and multiplies it by a funding
  tilt factor in `[0.7, 1.3]`, so funding can re-order and re-size the book but
  never manufacture conviction the base score did not already have. In `risk_off`,
  both de-risk to reserve; the carry tilt is explicitly subordinate to capital
  preservation.

## The funding tilt (the carry skill's distinguishing layer)

The funding proxy `f` (per hour, clamped to `[-1, 1]`) maps to a triangular tilt:

- **Favourable band** `f in [-0.05, 0.10]` -> `tilt = 1.0` (longs cheap or only
  mildly paying — best carry).
- **Overheated** `0.10 < f < 0.30` -> tilt decays 1 -> 0 (crowded longs we fade).
- **Faded** `f >= 0.30` -> `tilt = 0.0` (no-initiate).
- **Recovering** `-0.40 < f < -0.05` -> tilt decays 0 -> 1.
- **Capitulating** `f <= -0.40` -> `tilt = 0.0` (forced-unwind risk, no-initiate).

## When each outperforms

**regime-routed-bsc-alpha** tends to outperform when:

- Spot momentum and breadth are the dominant signal (clean trends in `risk_on` /
  `breakout`), where RSI/MACD-confirmed momentum captures the move directly.
- There is no meaningful or reliable derivatives-funding dislocation to exploit —
  funding sits near neutral across the universe, so a carry tilt adds noise rather
  than edge.
- You want the broadest factor coverage without depending on the funding proxy.

**funding-rate-carry-bsc** tends to outperform when:

- Perp funding is dispersed: some names are richly-positive (crowded, expensive
  longs worth fading) while others are negative/mildly positive (cheap longs worth
  holding). The tilt then re-ranks the book toward better-positioned carry.
- Crowded-long blow-offs are a risk — fading `funding >= 0.30` sidesteps names the
  derivatives crowd is most exposed to, improving drawdown behaviour vs the pure
  alpha rank.
- In chop, where pure momentum is weak, a funding edge (collect/avoid-paying
  funding) can be the marginal differentiator among similarly-scored candidates.

Both converge in deep `risk_off`: each rotates to the USDT reserve at the 0.2x
exposure multiplier, so funding attractiveness is irrelevant — capital
preservation and the same drawdown throttle / kill switch dominate.
