# Helm chart

A Helm packaging of the Guardrail Alpha stack. This complements the raw
manifests under [`deploy/k8s`](../k8s) — same topology, but templated and
configurable via `values.yaml`.

## Topology

Identical to `deploy/k8s`:

- **`guardrail-core`** — one pod running `agent` + `api` + `exporter` + `monitor`,
  all sharing a single `data` volume (SQLite event log + run report). The agent
  is the **sole writer**, so the Deployment is pinned to `replicas: 1` with a
  `Recreate` strategy — never scale it horizontally.
- **`guardrail-dashboard`** — a separate Deployment + Service that talks to the
  API over the `*-api` Service (the browser uses `NEXT_PUBLIC_API_URL`).
- Services: `*-api` (8080), `*-exporter` (9100), `*-dashboard` (3000).

## Install

```bash
helm install guardrail deploy/helm/guardrail -n guardrail --create-namespace
```

Upgrade after changing values:

```bash
helm upgrade guardrail deploy/helm/guardrail -n guardrail \
  --set agent.image.tag=v1.2.3 \
  --set api.image.tag=v1.2.3
```

Render locally without installing:

```bash
helm template guardrail deploy/helm/guardrail | less
helm lint deploy/helm/guardrail
```

Uninstall:

```bash
helm uninstall guardrail -n guardrail
```

## Values overview

| Key | Default | Purpose |
|---|---|---|
| `<svc>.image.repository` / `.tag` | `guardrail/<svc>` / `latest` | Image per service (`agent`, `api`, `exporter`, `monitor`, `dashboard`). |
| `<svc>.resources` | see `values.yaml` | CPU/memory requests + limits per container. |
| `data.databaseUrl` | `sqlite://data/guardrail_alpha.db` | `DATABASE_URL` for agent/api/exporter. |
| `data.report` | `data/run_report.json` | `GUARDRAIL_REPORT` for agent/exporter/monitor. |
| `data.mountPath` | `/app/data` | Mount path of the shared data volume. |
| `dashboard.apiUrl` | `http://localhost:8080` | `NEXT_PUBLIC_API_URL` (browser-facing). |
| `dashboard.enabled` | `true` | Toggle the dashboard Deployment + Service. |
| `api.port` | `8080` | API service/container port. |
| `exporter.port` | `9100` | Exporter (metrics) service/container port. |
| `dashboard.port` | `3000` | Dashboard service/container port. |
| `persistence.enabled` | `false` | `false` -> `emptyDir`; `true` -> a PVC for the shared data volume. |
| `persistence.size` | `1Gi` | PVC size when `persistence.enabled=true`. |
| `persistence.storageClass` | `""` | Optional storage class for the PVC. |
| `configs.enabled` / `.name` | `true` / `guardrail-configs` | Mount an optional ConfigMap of `configs/`. |
| `service.type` | `ClusterIP` | Service type for api/exporter/dashboard. |
| `securityContext` | non-root uid 10001 | Pod security context for the core pod. |

## Relationship to `deploy/k8s`

`deploy/k8s` holds the plain (kustomize) manifests — the canonical, easy-to-read
description of the deployment. This chart mirrors those exact resources but
parameterizes images, resources, env, ports, and the data-volume backing
(`emptyDir` vs PVC) through `values.yaml`. Pick one path:

- **kustomize** (`kubectl apply -k deploy/k8s/`) for a fixed, GitOps-friendly set.
- **Helm** (this chart) when you want per-environment overrides via `--set`/`-f`.

Both produce the same `guardrail-core` + `dashboard` topology. As with the raw
manifests, point Prometheus/Grafana (deployed via their own upstream charts) at
the exporter Service (`:9100/metrics`).
