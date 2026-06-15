# Guardrail Alpha — Guided Judge Walkthrough

A single, **offline-safe** narrated tour of the whole product. No API keys, no
network, no running services required. One command exercises the real tools end
to end and always exits 0 (every step is guarded).

```bash
bash scripts/judge_walkthrough.sh
```

> Companion script: [`scripts/judge_walkthrough.sh`](../../scripts/judge_walkthrough.sh).
> For the *live* stack (build + API + cockpit) use
> [`scripts/judge_quickstart.sh`](../../scripts/judge_quickstart.sh) instead — this
> walkthrough is the zero-setup, read-only version.

---

## What the walkthrough demonstrates

The script narrates six steps, printing a banner and a plain-English explanation
before each, then running the **actual** Guardrail tool (not a mock):

| Step | What runs | What it proves |
|------|-----------|----------------|
| 1. Demo data | `scripts/seed_demo_data.sh` if present, else uses existing `data/` | Everything downstream reads a local event-log DB + run report. Fully offline. |
| 2. Analytics | `python3 python-lab/analyze.py regime \| drawdown \| montecarlo \| ensemble-compare --all \| journal` | Real risk + decision analytics over the run's NAV curve and decision log. |
| 3. Report bundle | `python3 python-lab/analyze.py bundle` | Composes a self-contained HTML report folder (dossier + journal + ensemble), inline CSS, no CDN/JS. Prints where it landed. |
| 4. Skills | `scripts/lint_skills.sh` + the catalog from `skills/INDEX.json` | Each strategy skill's worked examples pass the **real** validator; the 5-skill catalog is printed with summaries. |
| 5. Proof | `scripts/verify_proof.sh` | A stdlib-only verifier re-derives the policy hash, report hash, and agent id and checks the on-chain contract + explorer URL — offline. |
| 6. Scenarios | `scripts/run_scenarios.sh` | The stress-scenario catalog: each failure mode and the guardrail response expected to fire (throttle / kill-switch / reduce-only / stop-loss). |

It closes with a **"what you just saw"** summary plus pointers to the cockpit and
the Next.js dashboard.

### Resilience contract

Every step is wrapped in a guard. If a tool is missing, a Python interpreter is
absent, or a step returns non-zero, the walkthrough prints a clear NON-FATAL
note and **continues** — it never aborts and always exits 0. This means a judge
can run it unattended on any checkout, including a partial one, and still get a
coherent, well-formed tour. It also runs correctly from any working directory
(paths resolve from the script's own location).

If `scripts/seed_demo_data.sh` does not exist (it is optional), the walkthrough
says so and proceeds with the data already in `data/`. Analytics automatically
target `data/guardrail_alpha.db` when present, otherwise `analyze.py`'s own
default — and on truly empty data they print a clean "no data" note rather than
crashing.

---

## The ~5-minute judge narrative

Read top to bottom; the script does the talking, but here is the story arc:

1. **"It's all local."** (~20s) — Step 1 confirms the demo event-log database
   and run report exist. No keys, no network. Everything that follows is
   reproducible on the judge's machine.

2. **"The agent's decisions are measurable."** (~90s) — Step 2 is the heart of
   the tour. *Regime* shows how exposure is routed by market regime. *Drawdown*
   shows the underwater curve and worst episodes. *Monte Carlo* bootstraps the
   NAV returns into VaR/CVaR tail risk and a probability of breaching the
   kill-switch drawdown. *Ensemble-compare* contrasts the blended book against
   each single skill (concentration / diversification / overlap). *Journal*
   prints the per-cycle reasoning — *why* the agent did what it did.

3. **"Here's the polished artifact."** (~30s) — Step 3 builds a browsable HTML
   report bundle and prints its path; open `data/reports/index.html` to read the
   full research dossier offline.

4. **"The strategies are real and validated."** (~40s) — Step 4 lints every
   skill's examples with the same validator the agent uses, then lists the
   5-skill catalog (regime-routed alpha, funding-rate carry, mean-reversion
   chop, trend/breakout momentum, volatility-targeted risk parity).

5. **"You don't have to trust us — verify it."** (~40s) — Step 5 independently
   re-derives the agent identity and report proof from first principles and
   checks the on-chain contract and explorer URL, all offline.

6. **"And it fails safe."** (~40s) — Step 6 walks the stress-scenario catalog,
   showing for each failure mode which guardrail protection is expected to fire.

End on the summary panel and, if you want the live experience, run
`scripts/judge_quickstart.sh` for the cockpit + API, or `cd dashboard && pnpm dev`
for the full Next.js dashboard.

---

## Prize-lane mapping

This single walkthrough touches every lane the product competes in:

- **Autonomous trading agent / core product** — Steps 2 & 6: regime routing,
  the analytics suite, and the stress-scenario → guardrail-response mapping show
  a real risk-managed agent, not a toy.
- **Risk management & safety** — Steps 2 (drawdown, Monte-Carlo tail risk) and 6
  (kill-switch / throttle / reduce-only / stop-loss) demonstrate the "guardrail"
  thesis end to end.
- **Verifiability / on-chain identity** — Step 5: the offline proof verifier
  re-derives the policy hash, report hash, and agent id and checks the BSC
  contract + explorer URL. Trust-minimized by construction.
- **Strategy / skills ecosystem** — Step 4: five validated, regime-complementary
  strategy skills plus an ensemble blender, all lint-clean.
- **Developer experience / judge-ability** — The whole tour: one offline command,
  guarded steps, never hard-fails, clear narration, and a self-contained HTML
  report you can open without a server.

---

## Pointers

- One entry point for everything: `bash scripts/guardrail.sh help`
- Live stack (build + API + cockpit): `bash scripts/judge_quickstart.sh`
- Static cockpit only (offline): `bash scripts/serve_cockpit.sh`
- Full dashboard: `cd dashboard && pnpm install && pnpm dev` → http://localhost:3000
- Report bundle (after running the walkthrough): `data/reports/index.html`
