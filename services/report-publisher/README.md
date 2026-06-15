# Guardrail Report Publisher

Renders the python-lab HTML report bundle (dossier · journal · ensemble) into a
published directory and writes an index, so a daily signed report set can be
served statically. Read-only over the event log + run report — it never trades.

It reuses the existing analytics (`guardrail_lab.report_bundle`, falling back to
`python3 python-lab/analyze.py bundle`) rather than re-implementing them.

## Run
```bash
# print the plan, write nothing (default)
python3 services/report-publisher/publisher.py --dry-run

# render the bundle to reports/published/
python3 services/report-publisher/publisher.py --write --out reports/published
```

Standard library only.
