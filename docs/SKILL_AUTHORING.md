# Authoring a Track-2 Strategy Skill

This guide explains how to add a new **Track-2 (Strategy Skills)** skill to
Guardrail: the required file layout, the `validate_example` contract every example
must satisfy, the helper scripts (`new_skill.sh`, `lint_skills.sh`), and the
shared regime model + risk envelope every skill must honour.

A Track-2 skill is a *backtestable trading-strategy specification* expressed as
data + prompts — **not** an executor. It turns CoinMarketCap (and
Guardrail-derived) market data into a regime-routed long/stable rotation over the
same fixed universe of 20 eligible BSC tokens, emits a decision payload (regime,
per-asset scores, target portfolio, entry/exit/heartbeat actions), and defers the
final word on every trade to the production Rust risk engine.

> A skill describes *what* the strategy would do. The Rust risk engine in
> `crates/risk-engine` remains the **final authority** — it validates, clamps, or
> rejects every LLM-proposed target before a swap ever happens. A skill can
> re-rank and re-size candidates, but it can never breach a risk limit.

---

## Quick start

```bash
# 1. Scaffold a new skill from the skeleton.
bash scripts/new_skill.sh my-new-strategy-bsc

# 2. Edit skills/my-new-strategy-bsc/* — customise the signal/tilt and replace
#    every <PLACEHOLDER>.

# 3. Validate the examples with the real validator.
bash scripts/lint_skills.sh
```

`new_skill.sh` copies `skills/_template`, substitutes the placeholder name token,
refuses to overwrite an existing directory, and prints the next steps.
`lint_skills.sh` runs the real `guardrail_lab.skill` validator over every skill's
`examples/` directory and exits non-zero if any example is invalid.

---

## File layout

Every skill is a top-level directory under `skills/` with this layout (the
skeleton in `skills/_template/` is exactly this shape):

```
my-new-strategy-bsc/
├── skill.yaml              # manifest: name, version, track, inputs, outputs, regimes, examples
├── strategy_spec.yaml      # the complete, backtestable strategy spec (single source of truth)
├── README.md               # quick-start summary
├── SKILL.md                # longer narrative + decision procedure + guardrails
├── prompts/
│   ├── system.md           # role + hard constraints for the strategy LLM
│   ├── strategy_generation.md  # step-by-step regeneration recipe
│   └── backtest_spec.md    # how to produce a defensible backtest
├── examples/               # one full signal -> decision payload per regime
│   ├── risk_on_example.json
│   ├── risk_off_example.json
│   ├── chop_example.json
│   └── breakout_example.json
└── tests/                  # required-output schema + smoke fixtures
    ├── test_strategy_schema.json
    └── test_outputs.json
```

The only section that meaningfully differs between skills is the **signal/tilt**
(section 4 of `strategy_spec.yaml`). The universe, regime model, allocator, and
risk envelope are shared and should be kept identical (document any deliberate
deviation in `strategy_spec.yaml`).

---

## The `validate_example` contract

Each `examples/*.json` file must produce an **empty issue list** from
`guardrail_lab.skill.validate_example`. The validator
(`python-lab/guardrail_lab/skill.py`) is standard-library only and shape-tolerant:
it searches the top level **and** the nested `computed`, `decision`, and `inputs`
scopes, so a full signal→decision payload validates the same as a minimal
`{ "market_regime": ..., "target_portfolio": [...] }` example.

An example is valid when **all** of the following hold:

1. **A market regime is present and non-empty** — under `market_regime` or
   `regime` (top level or in `computed` / `decision` / `inputs`).
2. **A target portfolio is present** — a non-empty list under `target_portfolio`
   or `portfolio`, where each position is an object carrying a non-empty string
   `symbol` and a numeric `weight_pct` (numbers or numeric strings are accepted).
3. **Risk weights do not over-allocate** — the sum of `weight_pct` is `<= 100`
   within a 1.0pp tolerance. The remainder is the held stable (USDT) reserve, so
   sums **below** 100 are expected and fine.
4. **Entry/exit are paired (only if a rules block is declared)** — if the example
   includes a `"rules": { ... }` object, or flat top-level `entry`/`exit` keys,
   then **both** `entry` and `exit` must be present and non-empty. If you omit the
   rules block entirely, this check is skipped.

Two helper functions matter:

- `load_skill_examples(skill_dir)` loads every `*.json` file in a skill's
  `examples/` directory, annotates each with a `_source` key (the file name),
  skips anything that is not a JSON object, and returns the list sorted by file
  name.
- `validate_example(example)` returns a list of human-readable issues, or `[]`
  when the example is well-formed.

### Common mistakes that fail validation

- Weights summing to **more than 100** (over-allocated). Leave the remainder as
  the USDT reserve; do not force the risk positions to total 100.
- A position missing its `symbol` or with a non-numeric `weight_pct`.
- Declaring a `rules` block (or a top-level `entry`/`exit`) with only one side.
- An empty `target_portfolio`, or a regime string that is empty/whitespace.

### Validate directly

```bash
cd python-lab && python3 -c "
from guardrail_lab.skill import load_skill_examples as L, validate_example as V
ex = L('../skills/my-new-strategy-bsc/examples')
assert ex and all(V(e) == [] for e in ex), 'examples invalid'
print('valid', len(ex))
"
```

Or, more simply, run `bash scripts/lint_skills.sh`, which does this for every
skill at once and reports per-skill PASS/FAIL.

---

## The helper scripts

### `scripts/new_skill.sh <name>`

Scaffolds a new skill from `skills/_template`:

- Validates that `<name>` is kebab-case and not the reserved `_template`.
- Refuses to overwrite an existing `skills/<name>` directory.
- Copies the skeleton and does a safe `sed` replacement of the placeholder name
  token across every file.
- Prints the next steps.

```bash
bash scripts/new_skill.sh funding-skew-bsc
```

### `scripts/lint_skills.sh`

Runs the **real** validator over every `skills/*/examples` directory:

- Imports `load_skill_examples` / `validate_example` from `python-lab/`.
- Prints `PASS`/`FAIL` per skill, listing each offending file and issue.
- Exits **non-zero** if any example is invalid or any skill that ships an
  `examples/` directory has no loadable examples.

```bash
bash scripts/lint_skills.sh
```

Set `PYTHON_BIN` to override the interpreter (defaults to `python3`).

---

## The shared regime model

Every skill classifies the market into one of four regimes using the same
top-down rules (mirroring `crates/strategy-engine/src/regime.rs::classify()`).
The first matching rule wins:

| Order | Regime | Condition | Exposure multiplier |
|-------|--------|-----------|---------------------|
| 1 | `breakout` | `breadth_pct >= 65 AND median_24h_return > 2 AND fear_greed >= 60` | 1.1 |
| 2 | `risk_on` | `breadth_pct >= 55 AND fear_greed >= 50` | 1.0 |
| 3 | `risk_off` | `breadth_pct <= 40 OR fear_greed <= 30 OR median_24h_return < -2` | 0.2 |
| 4 | `chop` | default (none of the above) | 0.5 |

Inputs: `breadth_pct` (% of risk assets with a positive 24h return),
`median_24h_return`, and the CMC Fear & Greed value (0..100). The exposure
multiplier scales the risk budget in the allocator.

---

## The shared risk envelope

All skills honour the same risk envelope, sourced from
`crates/risk-engine/src/policy.rs` and
`crates/strategy-engine/src/strategy_config.rs`:

- Per-name cap **17%** (policy max **18%**); stable reserve **15% target**
  (>= **10%** floor). Surplus over the cap falls back to the USDT reserve — never
  rejected.
- `min_score_to_enter` **0.65**, `min_score_to_hold` **0.50**,
  `max_positions` **5**, `max_new_position_pct` **12%**.
- Stop-loss **12%**, take-profit **25%** per position; rebalance only when target
  vs current drifts > **3%** of NAV.
- Drawdown throttle at **22%** total drawdown (block new buys, soft); kill switch
  latches at **24%** (halt trading).
- Execution: `require_quote_before_swap`, `twap_only`, `max_slippage_pct` **0.8**.
- Daily-trade requirement: >= **1** trade/day (a heartbeat <= **0.10%** NAV
  satisfies it when the book is flat).

In `risk_off`, de-risk to the USDT reserve regardless of how attractive the
signal looks — capital preservation outranks the signal.

---

## After you author a skill

1. Run `bash scripts/lint_skills.sh` and confirm your skill reports `PASS`.
2. Replace every `<PLACEHOLDER>` in `skill.yaml`, `README.md`, `SKILL.md`,
   `strategy_spec.yaml`, and the three prompts.
3. Add an entry to `skills/INDEX.json` and a row to `skills/README.md`, pulling
   all values from your real spec files (and counting the `examples/*.json` files
   for `examples_count`).
