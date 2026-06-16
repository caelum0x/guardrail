# position-sizer

Pure-Rust position sizing algorithms for trading systems. No external services,
no async, no other workspace crates — only `std`, `serde`, and `rust_decimal`.
Builds standalone via `--manifest-path`.

## Algorithms

| Module                | Function                    | Formula |
|-----------------------|-----------------------------|---------|
| `fixed_fractional`    | `fixed_fractional`          | `units = (equity · risk_fraction) / risk_per_unit` |
| `vol_target`          | `vol_target`                | `leverage = clamp(target_vol / asset_vol, 0, max_leverage)`; `notional = leverage · capital` |
| `kelly`               | `kelly_fraction`            | `f* = edge / odds = (b·p − q) / b`, scaled by a fractional multiplier and capped |
| `equal_risk`          | `equal_risk_contribution`   | `wᵢ = (1/σᵢ) / Σⱼ(1/σⱼ)` (inverse-vol / risk parity, diagonal covariance) |
| `decimal`             | `fixed_fractional_units`    | Exact `rust_decimal` fixed-fractional sizing with lot rounding |

Each algorithm lives in its own module with `#[cfg(test)]` known-value tests.
All sizing functions validate inputs at the boundary and return a typed
`SizingError` (no panics, no `NaN`/`Inf`).

## Usage

```rust
use position_sizer::{vol_target, VolTargetInput};

let out = vol_target(VolTargetInput {
    capital: 1_000_000.0,
    target_vol: 0.10,   // desired 10% vol
    asset_vol: 0.25,    // asset runs at 25% vol
    max_leverage: 2.0,
}).unwrap();
// out.leverage == 0.4, out.notional == 400_000.0
```

```rust
use position_sizer::kelly_fraction;
use position_sizer::KellyInput;

// p = 0.6 win prob, even-money odds, half-Kelly, capped at 20%.
let k = kelly_fraction(KellyInput {
    win_prob: 0.6,
    odds: 1.0,
    fraction: 0.5,
    cap: 0.2,
}).unwrap();
// k.full_kelly == 0.2, k.fraction_of_capital == 0.1
```

```rust
use position_sizer::equal_risk_contribution;

let weights = equal_risk_contribution(&[
    ("A".into(), 0.10),
    ("B".into(), 0.20),
]).unwrap();
// weights sum to 1; lower-vol asset gets the larger weight (2/3 vs 1/3).
```

## Build & test

```bash
cargo test --manifest-path /Users/arhansubasi/guardrail/crates/position-sizer/Cargo.toml
```

## Notes on the ERC model

`equal_risk_contribution` implements the closed-form inverse-volatility
solution, which equalises each asset's risk contribution under a **diagonal**
covariance matrix (assets treated as uncorrelated, or under an
equal-correlation assumption). For a full correlated risk-parity solve an
iterative (e.g. Newton or cyclical-coordinate) algorithm is required; that is
intentionally out of scope for this small, dependency-light crate.

## License

MIT
