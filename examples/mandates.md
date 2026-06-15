# Example mandates

The policy compiler turns plain English into a validated, hashed `RiskPolicy`.
The LLM may *propose* the text; the deterministic parser + validator are what
bind it — the model has no direct authority.

Compile any of these with:

```bash
cargo run -p guardrail-cli -- policy compile "<mandate>"
# or via the API
curl "http://localhost:8080/policy/compile?mandate=<url-encoded mandate>"
```

## Conservative

```
Trade CAKE and WBNB on BSC. Keep max drawdown at 18%, daily loss 5%,
max position 12%, stable reserve 20%, slippage 0.4%, kill switch at 22%,
at least 1 trade per day, no leverage.
```
Extracts: total drawdown 18%, daily 5%, position 12%, reserve 20%, slippage 0.4%,
kill switch 22%, allowlist [CAKE, WBNB], `borrow_without_policy` forbidden.

## Balanced

```
Trade CAKE, WBNB, BTCB on BSC. Max drawdown 22%, daily loss 7%,
max position 18%, stable reserve 12%, slippage 0.8%, kill switch 24%,
2 trades per day.
```

## Aggressive (still guard-railed)

```
Trade CAKE, WBNB, ETH, BTCB. Max drawdown 28%, daily loss 9%,
max position 22%, stable reserve 8%, slippage 1.0%, kill switch 30%.
```

## Validation failures (rejected by design)

These compile-fail because the validator enforces internal consistency:

```
max drawdown 30%, kill switch at 10%, trade CAKE     # kill switch < total cap
max position 10%, new position 20%, trade CAKE        # new cap > position cap
daily loss 40%, max drawdown 20%, trade CAKE          # daily > total cap
```
