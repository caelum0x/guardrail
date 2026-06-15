# Deployment

Guardrail Alpha ships four supported deployment paths, from a one-command local
stack to production Kubernetes. They all run the **same images** and the **same
topology** — they differ only in the orchestrator and the operational trade-offs.

## The topology (true for every path)

The trading **agent** is the **sole writer** of a SQLite event log
(`data/guardrail_alpha.db`) and a run report (`data/run_report.json`). Three
read-only sidecars consume that shared data:

- **api** — read-only JSON/HTTP API (`:8080`, `/health`).
- **exporter** — Prometheus metrics (`:9100/metrics`).
- **monitor** — watchdog / readiness checks.

A separate **dashboard** (Next.js, `:3000`) is a browser UI that talks to the
API. Optional **Prometheus** + **Grafana** scrape and visualize the exporter +
API. An optional **alert-relay** polls the API `/alerts` feed and forwards to
chat/email sinks (it holds no keys; secrets are injected at runtime).

Because the agent is a single writer, it is **never scaled horizontally**: one
agent process per data store. In Kubernetes the agent + sidecars co-locate in one
pod (`replicas: 1`, `Recreate`) sharing a volume.

## Deployment matrix

| Path | Where it lives | Best for | Persistence | Scaling | Monitoring | Effort |
|---|---|---|---|---|---|---|
| **Local — Compose** | [`deploy/compose`](../deploy/compose) | Dev, demos, evaluation, a full one-host run | Named Docker volume | Single host | Built-in (`monitoring` profile) | Lowest |
| **Kubernetes — kustomize** | [`deploy/k8s`](../deploy/k8s) | Production on an existing cluster, GitOps | PVC (swap from `emptyDir`) | Cluster (agent stays 1 replica) | Upstream Prometheus/Grafana charts | Medium |
| **Kubernetes — Helm** | [`deploy/helm`](../deploy/helm) | Production with per-environment overrides | PVC via `persistence.enabled` | Cluster (agent stays 1 replica) | Upstream Prometheus/Grafana charts | Medium |
| **Single host — systemd** | [`infra/systemd`](../infra/systemd) | A dedicated VM/bare-metal box, no containers | Host filesystem | Single host | Run Prometheus/Grafana separately | Medium |

### When to use which

- **Compose** — Start here. One command brings up the entire stack, including
  optional Prometheus/Grafana and the alert relay via profiles. Ideal for local
  development, a quick demo, or running the whole thing on a single machine.
- **kustomize (k8s)** — Use when you already operate Kubernetes and want plain,
  reviewable manifests checked into Git (GitOps). Fixed configuration, minimal
  templating.
- **Helm (k8s)** — Use on Kubernetes when you need per-environment configuration
  (image tags, resource sizing, `emptyDir` vs PVC, enabling the alert relay)
  through `values.yaml` and `--set` overrides.
- **systemd** — Use on a dedicated VM or bare-metal host where you do not want a
  container runtime: build the release binaries, install them, and run each
  component as a managed service.

---

## 1. Local — Docker Compose

The fastest path; everything in one file with opt-in profiles.

```bash
cp deploy/compose/.env.example deploy/compose/.env
docker compose -f deploy/compose/docker-compose.full.yml up --build
```

- Dashboard: <http://localhost:3000> · API: <http://localhost:8080/health> ·
  Exporter: <http://localhost:9100/metrics>
- Add monitoring: `--profile monitoring` (Prometheus `:9090`, Grafana `:3001`).
- Add alerting: `--profile alerts` (offline dry-run by default).

Full reference, ports, and profiles: [`deploy/compose/README.md`](../deploy/compose/README.md).

The repository-root [`docker-compose.yml`](../docker-compose.yml) is a minimal
core-only variant; `deploy/compose/docker-compose.full.yml` is the complete
superset with a named data volume, healthchecks, and the monitoring/alerts
profiles.

---

## 2. Kubernetes — kustomize

Plain manifests for an existing cluster. The agent + api + exporter + monitor run
as one `guardrail-core` pod sharing a `data` volume; the dashboard is its own
Deployment + Service.

```bash
# Build & push images to your registry first.
for s in agent api exporter monitor dashboard; do
  docker build -f infra/Dockerfile.$s -t guardrail/$s:latest .
  docker push guardrail/$s:latest
done

kubectl apply -k deploy/k8s/
kubectl -n guardrail get pods
kubectl -n guardrail port-forward svc/guardrail-api 8080:8080
kubectl -n guardrail port-forward svc/guardrail-dashboard 3000:3000
```

- The shared `data` volume defaults to `emptyDir` (ephemeral). Swap it for a
  `PersistentVolumeClaim` to retain the event log across restarts.
- Mount real `configs/` and secrets via a ConfigMap/Secret; the deployment
  references an optional `guardrail-configs` ConfigMap.
- Deploy Prometheus + Grafana via their upstream charts and point Prometheus at
  the `guardrail-exporter` Service (`:9100/metrics`); reuse the rules + dashboards
  under `infra/prometheus` and `infra/grafana`.

Details: [`deploy/k8s/README.md`](../deploy/k8s/README.md).

---

## 3. Kubernetes — Helm

The same Kubernetes topology, templated and configurable via `values.yaml`.

```bash
helm install guardrail deploy/helm/guardrail -n guardrail --create-namespace

# Per-environment overrides:
helm upgrade guardrail deploy/helm/guardrail -n guardrail \
  --set agent.image.tag=v1.2.3 \
  --set api.image.tag=v1.2.3 \
  --set persistence.enabled=true

# Render or lint without installing:
helm template guardrail deploy/helm/guardrail | less
helm lint deploy/helm/guardrail
```

Configurable knobs include per-service image repo/tag, resource requests/limits,
`data.databaseUrl` / `data.report`, the dashboard `apiUrl`, `emptyDir` vs PVC
(`persistence.enabled`), and the optional alert relay (`alertRelay.enabled`, with
sink credentials supplied through an existing Kubernetes Secret named in
`alertRelay.secretName` — the chart never inlines secrets).

Details: [`deploy/helm/README.md`](../deploy/helm/README.md).

Pick kustomize **or** Helm — both render the same `guardrail-core` + dashboard
resources. Use kustomize for a fixed GitOps set, Helm for per-environment
overrides.

---

## 4. Single host — systemd

For a dedicated VM or bare-metal host with no container runtime. Build the
release binaries, install them, and run each component as a systemd unit.

```bash
# Build the release binaries.
cargo build --release -p guardrail-agent -p guardrail-api -p guardrail-monitor

# Install binaries and the working tree (configs/, migrations/).
sudo install -D target/release/guardrail-agent   /usr/local/bin/guardrail-agent
sudo install -D target/release/guardrail-api      /usr/local/bin/guardrail-api
sudo install -D target/release/guardrail-monitor  /usr/local/bin/guardrail-monitor
sudo mkdir -p /opt/guardrail-alpha
sudo cp -r configs migrations /opt/guardrail-alpha/

# Install and enable the units.
sudo cp infra/systemd/guardrail-*.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now guardrail-agent guardrail-api guardrail-monitor
sudo systemctl status guardrail-agent
```

The unit files live in [`infra/systemd`](../infra/systemd). They set
`WorkingDirectory=/opt/guardrail-alpha`, run the binaries from
`/usr/local/bin`, and restart on failure. Adjust the agent's `--config`
(e.g. `configs/production.toml`) for your deployment, and run Prometheus +
Grafana as their own services pointed at the exporter.

---

## Secrets & configuration (all paths)

- **Never commit real secrets.** Every template here ships non-secret defaults
  and empty placeholders only.
- **Compose** — values come from `deploy/compose/.env` (git-ignored). Copy it
  from `.env.example`.
- **Kubernetes / Helm** — supply secrets through Kubernetes Secrets; the alert
  relay reads them as env vars referenced by its config (`env:VAR_NAME`). The
  Helm chart never inlines secret values.
- **systemd** — use an `EnvironmentFile=` directive or a drop-in to inject
  secrets; keep that file `root`-owned with `0600` permissions.
- The **agent is the sole writer** in every path — run exactly one agent per
  data store.

## Health & observability (all paths)

- API health: `GET /health` on `:8080`.
- Metrics: `GET /metrics` on the exporter `:9100` (and the API `:8080`).
- Prometheus scrape config + alert rules: [`infra/prometheus`](../infra/prometheus).
- Grafana datasource + dashboards: [`infra/grafana`](../infra/grafana).
