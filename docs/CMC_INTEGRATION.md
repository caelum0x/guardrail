# CMC Integration

The `cmc-client` crate owns *all* CoinMarketCap communication and nothing else.
Downstream crates depend on a trait, never on the wire format. CMC is the agent's
**data-in** layer; TWAK is the execution-out layer.

## `CmcDataSource` trait

The single contract every provider implements (`crates/cmc-client/src/lib.rs`):

```rust
#[async_trait]
pub trait CmcDataSource: Send + Sync {
    async fn latest_quotes(&self, assets: &[Asset]) -> Result<Vec<CmcQuote>, CmcError>;
    async fn ohlcv(&self, asset: &Asset, interval: Interval) -> Result<Vec<Candle>, CmcError>;
    async fn fear_greed(&self) -> Result<FearGreedSnapshot, CmcError>;
    async fn dex_liquidity(&self, asset: &Asset) -> Result<DexLiquidity, CmcError>;
    async fn token_security(&self, asset: &Asset) -> Result<TokenSecurity, CmcError>;
    async fn trending(&self) -> Result<Vec<TrendingToken>, CmcError>;
    async fn global_market(&self) -> Result<GlobalMarket, CmcError>;
}
```

Returned models (`models.rs`) are normalized Rust types: `CmcQuote`, `Candle`
(with `Interval` of 15m/1h/4h/1d), `FearGreedSnapshot` (0..100 + classification),
`DexLiquidity`, `TokenSecurity` (flags + safety score), `TrendingToken`,
`GlobalMarket`.

## Which data is used

Every method on the trait feeds the strategy/risk pipeline:

- **`latest_quotes`** — per-asset price, 24h return, volume; the core scoring
  input.
- **`ohlcv`** — candles for momentum/volatility features (`feature-engine`,
  `indicators`).
- **`fear_greed`** — sentiment input to the regime classifier (risk-on / chop /
  risk-off).
- **`dex_liquidity`** — BSC DEX liquidity, used by the risk engine's liquidity
  check (`crates/risk-engine/src/checks/liquidity.rs`).
- **`token_security`** — security flags fed into `RiskContext.security_flags` and
  enforced by `checks/security_flags.rs`.
- **`trending`** — discovery candidates.
- **`global_market`** — breadth/dominance context for regime classification.

`market-data::SnapshotBuilder` consumes a `CmcDataSource` and produces a
`MarketSnapshot`; strategy code only ever sees the normalized snapshot.

## Transports

Selection is driven by `CmcCfg` flags in `crates/common/src/config.rs`
(`use_mock`, `use_rest`, `use_mcp`, `use_x402`):

- **Mock** (`mock.rs`, `MockCmcClient`) — the default in paper mode
  (`configs/paper.toml`, `cmc.use_mock = true`). Deterministic, network-free.
- **REST** (`client.rs`, `CmcRestClient`) — implements `CmcDataSource` against
  the CMC Pro API. Constructed with an API key and timeout (empty key →
  `CmcError::MissingApiKey`); uses a `RateLimiter` (30 req/min) and `with_retry`
  for transient failures. Parsing is defensive: it navigates
  `serde_json::Value` and falls back to sane defaults rather than failing a whole
  snapshot on one missing field. Supporting modules: `endpoints.rs`,
  `rate_limit.rs`, `retry.rs`.
- **MCP** (`mcp.rs`, `CmcMcpClient`) — JSON-RPC 2.0 over HTTP to a CoinMarketCap
  **AI Agent Hub** MCP endpoint. Each `CmcDataSource` method maps to an MCP tool
  call (e.g. `cmc_latest_quotes`, `cmc_fear_and_greed`); parsing mirrors the
  defensive REST approach.
- **x402** (`x402.rs`) — pay-per-request for premium endpoints.

`agent-runtime::build_data_source` picks the live REST client when
`cmc.use_mock` is false **and** `CMC_API_KEY` is set; otherwise it falls back to
`MockCmcClient` so the agent always runs.

## x402 (402 pay-and-retry)

`CmcRestClient::get` runs the full 402 retry loop (`pay_and_retry`,
`client.rs`): on an HTTP 402 it parses the advertised `PaymentRequirements`,
builds an unsigned `PaymentPayload::from_requirements`, and replays the request
with the `X-PAYMENT` header. Crucially, the signature is **not** produced here —
the payload's canonical `authorization_json()` is signed by TWAK
(`twak-client::x402::sign_authorization`), so keys never enter `cmc-client`.
Gated by `CMC_X402_ENABLED` (`x402::is_enabled`); payer/signature supplied via
`CMC_X402_FROM` / `CMC_X402_SIGNATURE`. See
[TWAK_INTEGRATION.md](TWAK_INTEGRATION.md) for the signing side.

## Mock (`MockCmcClient`)

`mock.rs` is a deterministic `CmcDataSource` for paper trading and tests. Values
are derived from a symbol hash and a monotonic tick, so runs are reproducible.
Numbers are shaped to exercise the strategy and risk paths: stables pin near $1,
non-stables get symbol-derived prices with constructive momentum and a few clear
leaders so the regime classifier sees breadth and the scorer separates names.
Construct with `MockCmcClient::new()` or `with_fear_greed(value)` to drive the
sentiment input.

## The CMC Skill: `skills/cmc-regime-routed-alpha`

A companion CMC Skill artifact (`skill.yaml`, name `regime-routed-bsc-alpha`)
that documents how the agent turns CMC data into a strategy. Declared inputs:
`cmc_quotes`, `cmc_ohlcv`, `cmc_dex_liquidity`, `cmc_fear_greed`, `cmc_trending`,
`eligible_asset_list`. Declared outputs: `market_regime`, `asset_scores`,
`target_portfolio`, `entry_rules`, `exit_rules`, `risk_policy`. Ships
`strategy_spec.yaml`, prompts (`system.md`, `strategy_generation.md`,
`backtest_spec.md`), regime examples (`examples/`), and schema tests
(`tests/`) — mirroring the Rust regime/scoring pipeline.

## "Best Use of Agent Hub" map

| Criterion | Where it lives |
|---|---|
| **Breadth of CMC data** | All seven `CmcDataSource` methods used: quotes, OHLCV, Fear & Greed, DEX liquidity, token security, trending, global. |
| **Agent Hub / MCP** | `CmcMcpClient` (`mcp.rs`) talks JSON-RPC to the CMC AI Agent Hub MCP endpoint; one tool call per data method. |
| **x402 monetized access** | `pay_and_retry` 402 loop + TWAK-signed `X-PAYMENT` payload. |
| **Skill packaging** | `skills/cmc-regime-routed-alpha` ships a reusable CMC Skill spec, prompts, examples, and tests. |
| **Clean abstraction** | One trait, four transports; strategy never sees raw CMC; defensive parsing keeps a snapshot alive on partial data. |
| **Offline reproducibility** | `MockCmcClient` keeps paper mode deterministic and key-free. |
