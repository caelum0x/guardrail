# Quant Suite

Guardrail's quant tools are pure, offline-safe building blocks. Each is a
standalone Rust crate, exposed five ways so it's reachable from anywhere:

**crate → API route → dashboard page → SDK (TS/Python/Go) → CLI subcommand**

All computation is deterministic and read-only; nothing here touches the trading
loop or holds keys.

## The tools

| Tool | Crate | API | Page | CLI | What it does |
|---|---|---|---|---|---|
| Indicators | `ta-signals` | `GET /ta` | `/ta-studio` | `ta` | SMA, EMA, RSI, MACD, Bollinger, ATR, Stochastic, OBV, ADX, VWAP over a series |
| Swap cost | `fee-model` | `GET /fees` | `/fees` | `fees` | Gas + constant-product price impact + linear slippage + protocol fee |
| Position sizing | `position-sizer` | `GET /sizer` | `/sizer` | `size` | Fixed-fractional, volatility-target, Kelly, equal-risk-contribution |
| Order book | `orderbook` | `GET /orderbook` | `/orderbook` | `book` | Price-time-priority limit/market matching engine |
| PnL attribution | `pnl-attribution` | `GET /pnl` | `/pnl` | `pnl` | Average-cost realized/unrealized PnL per symbol from a fill stream |
| Correlation | `correlation` | `GET /correlation` | `/correlation` | `corr` | Pearson correlation/beta + a named-series correlation matrix |

The `/quant` dashboard page indexes all six; `clients/web-lite/quant.html` is a
zero-build playground for them.

## Examples

### CLI (offline, no API needed)

```bash
cargo run -p guardrail-cli -- ta --indicator rsi --series 44,44.3,44.1,43.6,44.3,44.8 --period 5
cargo run -p guardrail-cli -- fees --notional 25000 --quantity 12 --side buy
cargo run -p guardrail-cli -- size --method kelly --win-prob 0.6 --odds 1.5
cargo run -p guardrail-cli -- book --orders "s,limit,101,5;b,market,,6"
cargo run -p guardrail-cli -- pnl --fills "CAKE,buy,10,2;CAKE,sell,4,3" --marks "CAKE:3"
cargo run -p guardrail-cli -- corr --series "BTC:0.01,-0.02,0.03;ETH:0.012,-0.018,0.025"
```

### API (read-only, port 8080)

```bash
curl -fsS 'http://127.0.0.1:8080/ta?indicator=rsi&series=44,44.3,44.1,43.6,44.3&period=5'
curl -fsS 'http://127.0.0.1:8080/fees?notional_usd=25000&quantity=12&side=buy'
curl -fsS 'http://127.0.0.1:8080/sizer?method=kelly&win_prob=0.6&odds=1.5'
curl -fsS 'http://127.0.0.1:8080/orderbook?orders=s,limit,101,5;b,market,,6'
curl -fsS 'http://127.0.0.1:8080/pnl?fills=CAKE,buy,10,2;CAKE,sell,4,3&marks=CAKE:3'
curl -fsS 'http://127.0.0.1:8080/correlation?series=BTC:0.01,-0.02,0.03;ETH:0.012,-0.018,0.025'
```

### SDKs

```ts
// TypeScript (@guardrail/client)
const c = new GuardrailClient({ baseUrl: "http://127.0.0.1:8080" });
await c.ta({ indicator: "rsi", series: [44, 44.3, 44.1], period: 5 });
await c.fees({ notionalUsd: 25000, quantity: 12, side: "buy" });
await c.sizer({ method: "kelly", win_prob: 0.6, odds: 1.5 });
await c.pnl("CAKE,buy,10,2;CAKE,sell,4,3", "CAKE:3");
await c.correlation("BTC:0.01,-0.02,0.03;ETH:0.012,-0.018,0.025");
```

```python
# Python (guardrail_client)
c = GuardrailClient(base_url="http://127.0.0.1:8080")
c.ta("rsi", [44, 44.3, 44.1], period=5)
c.fees(notional_usd=25000, quantity=12, side="buy")
c.sizer("kelly", win_prob=0.6, odds=1.5)
c.pnl("CAKE,buy,10,2;CAKE,sell,4,3", "CAKE:3")
c.correlation("BTC:0.01,-0.02,0.03;ETH:0.012,-0.018,0.025")
```

```go
// Go (github.com/guardrail-alpha/guardrail-go)
c := guardrail.NewClient("http://127.0.0.1:8080")
c.TA(ctx, "rsi", []float64{44, 44.3, 44.1}, 5, 0)
c.Fees(ctx, map[string]string{"notional_usd": "25000", "side": "buy"})
c.Sizer(ctx, "kelly", map[string]string{"win_prob": "0.6", "odds": "1.5"})
c.PnL(ctx, "CAKE,buy,10,2;CAKE,sell,4,3", "CAKE:3")
c.Correlation(ctx, "BTC:0.01,-0.02,0.03;ETH:0.012,-0.018,0.025")
```

## Design notes

- **Pure & deterministic** — no I/O, no clocks, no randomness; same inputs →
  same outputs. Indicator outputs are input-aligned with `NaN`/`null` warmup.
- **Exact money math** — `fee-model`, `position-sizer` (decimal mode),
  `pnl-attribution`, and `orderbook` use `rust_decimal`; indicators and
  `correlation` use `f64`.
- **Degrade, don't panic** — zero-variance correlation returns `0.0` (not NaN);
  a missing/non-positive price is skipped; division guards everywhere.
- **Tested** — each crate ships `#[cfg(test)]` known-value tests (orderbook 8,
  ta-signals 24, position-sizer 44, fee-model 17, pnl-attribution 6,
  correlation 7) plus the API-handler parse tests.
