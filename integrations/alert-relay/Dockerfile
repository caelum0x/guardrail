# Guardrail alert relay container.
#
# A tiny, dependency-free image: the relay uses only the Python standard
# library, so a slim base with no pip install step is all that is required.
#
# Runtime behavior is configured entirely via environment variables and the
# mounted config; the image itself contains NO secrets. Secrets are supplied
# at run time as env vars referenced by the config (env:VAR_NAME).
#
# Build:
#   docker build -t guardrail-alert-relay integrations/alert-relay
#
# Run (live, real delivery -- requires sink secrets in the environment):
#   docker run --rm \
#     -e GUARDRAIL_API_BASE_URL=http://host.docker.internal:8080 \
#     -e GUARDRAIL_SLACK_WEBHOOK_URL=... \
#     -v "$PWD/configs/alerts.json:/config/alerts.json:ro" \
#     guardrail-alert-relay
#
# The default command runs the continuous LIVE loop. Override the command to
# get a one-shot dry-run smoke test instead:
#   docker run --rm guardrail-alert-relay python3 relay.py --once --dry-run
FROM python:3-slim

# Do not write .pyc files or buffer stdout: logs should stream immediately.
ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1

# Path to the config the relay should load. Override with -e CONFIG_PATH=...
# and a bind-mounted file for production use.
ENV CONFIG_PATH=/app/configs/alerts.example.json

WORKDIR /app

# Copy only what the relay needs. The example config is baked in as a safe
# default; mount a real config over /config for production.
COPY integrations/alert-relay/relay.py integrations/alert-relay/sinks.py /app/
COPY configs/alerts.example.json /app/configs/alerts.example.json

# Run as a non-root user; the relay never needs elevated privileges.
RUN useradd --create-home --uid 10001 relay
USER relay

# Default to the continuous LIVE loop, reading the config from CONFIG_PATH.
# Secrets are resolved from the environment at runtime; none are baked in.
ENTRYPOINT ["python3", "relay.py"]
CMD ["--live", "--config", "/app/configs/alerts.example.json"]
