# Examples

Hands-on examples for Guardrail Alpha. All paper-mode flows are deterministic
and run offline (no keys, no network).

| File | What it shows |
|---|---|
| [mandates.md](./mandates.md) | Natural-language mandates and the policies they compile to |
| [api-cookbook.md](./api-cookbook.md) | `curl` recipes for every API endpoint |
| [cli-cookbook.md](./cli-cookbook.md) | `guardrail-cli` command recipes |
| [sample-policy.json](./sample-policy.json) | A compiled risk policy artifact |

## Fastest path

```bash
# One command exercises the whole pipeline:
./scripts/demo.sh

# Or step through it:
cargo run -p guardrail-doctor                      # preflight
cargo run -p guardrail-cli -- policy compile "Trade CAKE, max drawdown 20%, kill switch 25%"
GUARDRAIL_CYCLES=3 cargo run -p guardrail-agent -- --config configs/paper.toml
cargo run -p guardrail-cli -- report
```
