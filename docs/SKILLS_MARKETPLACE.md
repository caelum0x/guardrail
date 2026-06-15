# Skills Marketplace

Track-2 strategy skills are first-class, discoverable, and runnable end-to-end.

## `crates/skill-loader`

A runtime registry that parses `skills/INDEX.json` into a typed `SkillCatalog`
(`SkillEntry` per skill) and loads each skill's `strategy_spec.yaml` into a
`SkillSpec` (typed `SpecHeader` + dynamic body). Validates that catalog paths +
spec files exist; never panics on missing files. See
`crates/skill-loader/src/{catalog,spec,error}.rs`.

## API routes (guardrail-api)

| Route | Purpose |
|---|---|
| `GET /skills` | Catalog: count + ids + entries (from `skills/INDEX.json`). |
| `GET /skills/{id}` | One skill's detail — catalog entry + spec sections/description (via `skill-loader`). |
| `GET /skills/{id}/backtest?preset=` | Runs the real strategy+risk+portfolio backtest over the eligible universe with the preset, contextualized by the skill. |

Handlers: `apps/guardrail-api/src/skills.rs` and `src/skill_detail.rs`.

## Dashboard

- `dashboard/src/app/skills/page.tsx` — marketplace: a card per skill (name,
  thesis, regimes, example count) linking to detail.
- `dashboard/src/app/skills/[id]/page.tsx` — detail: spec summary + an on-demand
  backtest panel (`/skills/{id}/backtest?preset=balanced`).

## The skills themselves

Six Track-2 skills live under `skills/` (regime-routed alpha, funding-carry,
mean-reversion/chop, breakout/momentum, volatility-targeted risk-parity,
social-sentiment-momentum), plus `skills/ensemble.json` (the regime-routed blend)
and `skills/_template/` (a validator-clean starting point). Every example
validates against `python-lab/guardrail_lab/skill.py::validate_example`.
