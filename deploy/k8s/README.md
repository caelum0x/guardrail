# Kubernetes deployment

Production manifests for Guardrail Alpha. For local/dev, prefer `docker compose`
(see repo root `docker-compose.yml`).

## Topology

The agent and its read-only sidecars (`api`, `exporter`, `monitor`) share the
SQLite event log + run report, so they run as **one pod** (`guardrail-core`)
with a shared `data` volume. The agent is the **sole writer** — never scale it
horizontally (`replicas: 1`, `Recreate`). The `dashboard` runs as its own
Deployment and talks to the API over the `guardrail-api` Service.

| Resource | Purpose |
|---|---|
| `namespace.yaml` | `guardrail` namespace |
| `core-deployment.yaml` | agent + api + exporter + monitor (shared `data`) |
| `services.yaml` | `guardrail-api` (8080), `guardrail-exporter` (9100) |
| `dashboard.yaml` | dashboard Deployment + Service (3000) |
| `kustomization.yaml` | bundles the above |

## Build & push images

```bash
for s in agent api exporter monitor dashboard; do
  docker build -f infra/Dockerfile.$s -t guardrail/$s:latest .
  docker push guardrail/$s:latest   # to your registry
done
```

## Apply

```bash
kubectl apply -k deploy/k8s/
kubectl -n guardrail get pods
kubectl -n guardrail port-forward svc/guardrail-api 8080:8080
kubectl -n guardrail port-forward svc/guardrail-dashboard 3000:3000
```

## Notes

- The `data` volume is an `emptyDir` (ephemeral). Swap it for a `PersistentVolumeClaim`
  to retain the event log across restarts.
- Mount real `configs/` and secrets via a ConfigMap/Secret (the deployment
  references an optional `guardrail-configs` ConfigMap).
- Prometheus + Grafana: deploy via their upstream Helm charts and point
  Prometheus at the `guardrail-exporter` Service (`:9100/metrics`); reuse the
  scrape rules + dashboards under `infra/prometheus` and `infra/grafana`.
