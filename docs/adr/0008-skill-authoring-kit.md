# ADR 0008 — Skill authoring kit: template + validator contract

- Status: Accepted
- Date: 2026-06-14

## Context

Track-2 strategy skills must stay interchangeable: each is a *backtestable
strategy specification expressed as data + prompts*, sharing one universe, one
regime model, and one risk envelope, differing only in the signal/tilt. Without
an enforced shape, new skills drift — divergent file layouts, over-allocated
example portfolios, or examples that silently fail to load — and the ensemble
(ADR 0006) can no longer treat them uniformly.

## Decision

Ship a **skill authoring kit** with two halves:

1. **A template + scaffolder.** `skills/_template/` defines the canonical
   layout (`skill.yaml`, `strategy_spec.yaml`, `README.md`, `SKILL.md`,
   `prompts/`, `examples/` per regime, `tests/`). `scripts/new_skill.sh <name>`
   copies it, enforces a kebab-case name, refuses to overwrite an existing
   directory, and substitutes the placeholder name token.

2. **A real validator contract.** Every `examples/*.json` must produce an empty
   issue list from `guardrail_lab.skill.validate_example` (standard-library
   only, shape-tolerant across top-level / `computed` / `decision` / `inputs`
   scopes). An example is valid iff: a non-empty regime is present; a non-empty
   `target_portfolio` of `{symbol, weight_pct}` positions is present; the
   `weight_pct` sum is `<= 100` (the remainder is the USDT reserve); and, *only
   if a rules block is declared*, `entry` and `exit` are both present.
   `scripts/lint_skills.sh` runs this validator over every skill and exits
   non-zero on any failure.

## Consequences

- Adding a skill is `new_skill.sh` → edit the signal/tilt → `lint_skills.sh`
  must report PASS; the kit makes "is this skill well-formed?" a one-command,
  machine-checkable question.
- The validator is the *same* code the analytics and ensemble rely on, so
  passing lint guarantees a skill is consumable by the meta-allocator.
- Sub-100 weight sums are expected and accepted — the contract treats the
  remainder as cash, never forcing risk positions to total 100.
- Trade-off: enforcing a shared shape constrains exotic strategies; deliberate
  deviations must be documented in `strategy_spec.yaml`.
- See `docs/SKILL_AUTHORING.md` for the full file layout, regime model, and
  risk envelope.
