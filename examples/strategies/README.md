# Strategy cookbook

Five worked example strategies for Guardrail Alpha. Each is a plain-English
**mandate** that the policy compiler turns into a validated, hashed `RiskPolicy`,
paired with a recommended **preset** that tunes day-to-day sizing.

- The **mandate** is the hard contract: drawdown brakes, position caps, the stable
  floor, slippage limits, and the kill switch. Compiled and validated
  deterministically — see `crates/policy-compiler/src/parser.rs` for the exact
  phrases recognized.
- The **preset** (`configs/strategy_presets.json`) sets the soft knobs: entry/hold
  score thresholds, max simultaneous positions, and a target stable reserve.
- Sizing across market conditions is governed by the regime classifier
  (`crates/strategy-engine/src/regime.rs`), which routes the whole strategy via a
  per-regime exposure multiplier: RiskOn `x1.0`, Breakout `x1.1`, Chop `x0.5`,
  RiskOff `x0.2`.

All mandates stay inside the 20 eligible BSC tokens
(`configs/eligible_assets.bsc.json`).

## The strategies

| Strategy | Thesis | Preset | Key risk limits (drawdown / position / stable / kill) |
|---|---|---|---|
| [conservative-core](./conservative-core.md) | Liquid majors + big stable buffer; survive first | `conservative` | 15% / 12% / 30% / 18% |
| [momentum-rotator](./momentum-rotator.md) | Rotate into top-scoring large-caps | `balanced` | 22% / 18% / 12% / 26% |
| [sentiment-contrarian](./sentiment-contrarian.md) | Buy washed-out liquid alts when fear is high | `aggressive` | 28% / 20% / 8% / 32% |
| [defensive-stablefirst](./defensive-stablefirst.md) | Capital preservation; heavy stables, tiny positions | `conservative` | 10% / 8% / 40% / 12% |
| [breakout-rider](./breakout-rider.md) | Concentrate into high-beta names during trends | `balanced` | 25% / 20% / 10% / 28% |

## Compile and run any strategy

Every flow below is paper-mode and deterministic (no keys, no network).

```bash
# 1. Compile the natural-language mandate into a validated policy + hash.
cargo run -p guardrail-cli -- policy compile "<mandate from the table / .md file>"

# 2. Backtest it against buy-and-hold with the recommended preset.
cargo run -p guardrail-cli -- backtest --steps 60 --preset <preset>

# 3. Walk-forward across sentiment-driven windows.
cargo run -p guardrail-cli -- walk-forward --windows 6 --steps 30

# 4. Compare all presets side by side at a chosen fear/greed level.
cargo run -p guardrail-cli -- compare --steps 60 --fear-greed 70

# Research binary (sentiment sweep / walk-forward):
cargo run -p guardrail-sim -- --steps 60 --preset <preset>
cargo run -p guardrail-sim -- --walk-forward --windows 6 --steps 30 --preset <preset>
```

Example, end to end, for `conservative-core`:

```bash
cargo run -p guardrail-cli -- policy compile "Trade BTCB, ETH and WBNB on BSC. Keep max drawdown at 15%, daily loss 4%, max position 12%, stable reserve 30%, slippage 0.4%, kill switch at 18%, at least 1 trade per day, no leverage."
cargo run -p guardrail-cli -- backtest --steps 60 --preset conservative
```

## Machine-readable index

[`index.json`](./index.json) lists every strategy as
`{ name, mandate, preset, file }` for tooling and CI.

## See also

- [../mandates.md](../mandates.md) — mandate parsing details and rejected
  (validation-failing) examples.
- [../cli-cookbook.md](../cli-cookbook.md) — the full CLI command surface.
