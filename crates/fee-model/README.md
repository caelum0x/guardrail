# fee-model

Pure-Rust estimation of the **all-in cost of a swap**. Decimal-exact money math
(`rust_decimal`), no network, no external services — just real formulas.

## Cost components

| Component  | Formula                                                        | Module        |
|------------|---------------------------------------------------------------|---------------|
| Gas        | `gas_units * gas_price_gwei * 1e-9 * native_usd`              | `gas`         |
| Slippage   | `notional * (notional/(liquidity+notional) + linear_bps/1e4)` | `slippage`    |
| Protocol   | `notional * protocol_fee_bps / 1e4`                           | `fee`         |

The slippage term combines a **constant-product price impact**
`N / (L + N)` (monotonic, 0 at zero notional, → 1 as the order dwarfs the pool)
with a configurable **linear slippage** in basis points for spread/MEV padding.

## Output

`SwapCostModel::estimate()` returns a `CostBreakdown`:

```rust
pub struct CostBreakdown {
    pub gas_usd: Decimal,
    pub slippage_usd: Decimal,
    pub fee_usd: Decimal,
    pub total_usd: Decimal,
    pub effective_price: Decimal,      // per-unit price after all costs
    pub total_cost_fraction: Decimal,  // total_usd / notional
}
```

For a **buy**, costs are added to notional before dividing by quantity (you pay
more per unit); for a **sell** they are subtracted (you receive less per unit).

## Usage

```rust
use fee_model::{SwapCostModel, SwapSide};
use rust_decimal::Decimal;

let model = SwapCostModel::builder()
    .notional_usd(Decimal::from(10_000))
    .quantity(Decimal::from(5))
    .side(SwapSide::Buy)
    .gas(Decimal::from(150_000), Decimal::from(25), Decimal::from(2_000))
    .pool_liquidity_usd(Decimal::from(990_000))
    .linear_slippage_bps(Decimal::from(5))
    .protocol_fee_bps(Decimal::from(30))
    .build();

let cost = model.estimate();
// gas 7.50 + slippage 105.00 + fee 30.00 = total 142.50
// effective_price = (10_000 + 142.50) / 5 = 2_028.50
```

## Build & test (standalone)

This crate declares an empty `[workspace]` so it builds independently of the
parent workspace:

```bash
cargo build --manifest-path crates/fee-model/Cargo.toml
cargo test  --manifest-path crates/fee-model/Cargo.toml
```

## Design notes

- All math is `Decimal` — no `f64`, so penny-level results are exact.
- Each component is its own struct (`GasParams`, `SlippageParams`, `FeeParams`)
  and is independently testable; `SwapCostModel` composes them.
- Division guards: zero quantity → `effective_price = 0`; zero notional →
  `total_cost_fraction = 0`. No panics on degenerate input.
