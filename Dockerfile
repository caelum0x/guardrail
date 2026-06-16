# Root Docker build for the read-only Guardrail Alpha API.
#
# Lives at the repo root so Render's default Dockerfile path (./Dockerfile) works
# for a plain Docker web service AND the render.yaml Blueprint. Mirrors
# infra/Dockerfile.api, but the healthcheck honors Render's injected $PORT.
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p guardrail-api

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/guardrail-api /usr/local/bin/guardrail-api
RUN useradd --create-home --uid 10002 guardrail \
    && mkdir -p /app/data \
    && chown -R guardrail /app
# Seed the read-only API with a deterministic paper dataset so the deployed
# dashboard shows real data on first load (ephemeral on free tier; overridden
# if a real agent ever writes to a mounted disk here).
COPY --chown=guardrail:guardrail deploy/seed/guardrail_alpha.db /app/data/guardrail_alpha.db
COPY --chown=guardrail:guardrail deploy/seed/run_report.json /app/data/run_report.json
USER guardrail
ENV RUST_LOG=info
# Render injects $PORT; the API's bind_addr honors PORT (host fixed to 0.0.0.0).
EXPOSE 8080
HEALTHCHECK --interval=15s --timeout=3s --retries=10 \
    CMD curl -fsS "http://localhost:${PORT:-8080}/health" || exit 1
ENTRYPOINT ["guardrail-api"]
