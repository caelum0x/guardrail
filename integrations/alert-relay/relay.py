#!/usr/bin/env python3
"""Guardrail alert relay.

A standalone, standard-library-only daemon that polls the (read-only)
Guardrail API ``/alerts`` endpoint (and optionally ``/readiness``), dedupes
alerts it has already seen, and forwards each new alert to configured chat
sinks: Telegram, Discord, and a generic webhook.

Design goals:
  * Zero third-party dependencies (urllib + json from the stdlib).
  * Offline-safe: ``--dry-run`` is the DEFAULT and makes no network calls
    to sinks; it prints what would be sent.
  * Crash-proof: a down API, malformed JSON, or a failing sink never raises
    out of the loop. The relay logs a clear message and keeps going.
  * Secrets never live in code or config: sink tokens are read from env vars
    referenced by name in the config.

Usage:
    python3 relay.py --once --dry-run        # single offline poll (default mode)
    python3 relay.py --once --live           # single real poll + real delivery
    python3 relay.py                         # continuous dry-run loop
    python3 relay.py --live --config my.json # continuous live loop

The API contract this relay consumes (see apps/guardrail-api):
    GET /alerts -> {
        "status": "clear|warning|critical",
        "counts": {"critical": int, "warning": int, "total": int},
        "alerts": [ {"kind": str, "severity": str, "message": str}, ... ],
        "inputs": { ... }
    }
    GET /readiness -> {"status": "ready|blocking", "blocking": int,
                        "checks": [ {"id","label","status","detail"}, ...]}
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import time
import urllib.error
import urllib.request

import sinks as sinks_module

# Resolve the config path relative to this file by default so the relay works
# regardless of the caller's working directory.
_THIS_DIR = os.path.dirname(os.path.abspath(__file__))
_REPO_ROOT = os.path.abspath(os.path.join(_THIS_DIR, os.pardir, os.pardir))
DEFAULT_CONFIG_PATH = os.path.join(_REPO_ROOT, "configs", "alerts.example.json")

# Ordered severity ranks for threshold filtering.
SEVERITY_RANK = {"info": 0, "warning": 1, "critical": 2}

# How long to wait on the API before treating it as unreachable.
API_TIMEOUT = 10


def log(message: str) -> None:
    """Emit a timestamped log line to stdout (line-buffered, offline-safe)."""
    stamp = time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime())
    print(f"[{stamp}] {message}", flush=True)


# ---------------------------------------------------------------------------
# Configuration loading
# ---------------------------------------------------------------------------

def load_config(path: str) -> dict:
    """Load and validate relay config from JSON. Raises on fatal problems.

    Validation is deliberately strict at this boundary (fail fast) because a
    bad config is an operator error, not a runtime condition to absorb.
    """
    if not os.path.exists(path):
        raise FileNotFoundError(f"config not found: {path}")
    try:
        with open(path, "r", encoding="utf-8") as handle:
            config = json.load(handle)
    except json.JSONDecodeError as exc:
        raise ValueError(f"config is not valid JSON ({path}): {exc}") from exc

    if not isinstance(config, dict):
        raise ValueError("config root must be a JSON object")

    api = config.get("api")
    if not isinstance(api, dict) or not api.get("base_url"):
        raise ValueError("config.api.base_url is required")

    sinks = config.get("sinks")
    if not isinstance(sinks, list):
        raise ValueError("config.sinks must be a list")

    return config


def resolve_secret(value: str) -> str:
    """Resolve a config secret reference into a real value.

    A value of the form ``env:VAR_NAME`` is read from the environment. Any
    other value is returned as-is (useful for non-secret fields such as a
    chat id). Missing env vars resolve to an empty string so dry-run never
    needs real secrets; live delivery will then report the sink as misconfigured.
    """
    if isinstance(value, str) and value.startswith("env:"):
        return os.environ.get(value[len("env:"):], "")
    return value if isinstance(value, str) else ""


def build_sinks(config: dict, *, dry_run: bool) -> list:
    """Construct enabled sinks from config, resolving secrets from env.

    Disabled or malformed sink entries are skipped with a warning rather than
    aborting the relay.
    """
    timeout = int(config.get("sink_timeout_seconds", sinks_module.DEFAULT_TIMEOUT))
    built = []
    for entry in config.get("sinks", []):
        if not isinstance(entry, dict):
            log("skipping malformed sink entry (not an object)")
            continue
        if not entry.get("enabled", False):
            continue
        kind = entry.get("kind", "")
        # Resolve known secret-bearing fields through resolve_secret.
        settings = {}
        for key, raw in entry.items():
            if key in ("kind", "enabled"):
                continue
            settings[key] = resolve_secret(raw) if isinstance(raw, str) else raw
        try:
            sink = sinks_module.build_sink(
                kind, settings, dry_run=dry_run, timeout=timeout
            )
        except ValueError as exc:
            log(f"skipping sink: {exc}")
            continue
        built.append(sink)
    return built


# ---------------------------------------------------------------------------
# API polling
# ---------------------------------------------------------------------------

def fetch_json(url: str, timeout: int = API_TIMEOUT):
    """GET a URL and parse JSON. Returns (data, error_message).

    On success ``error_message`` is None. On any failure ``data`` is None and
    ``error_message`` is a human-readable reason. Never raises: the API being
    down is an expected runtime condition, not an exception.
    """
    request = urllib.request.Request(url, method="GET")
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = response.read()
    except urllib.error.HTTPError as exc:
        return None, f"HTTP {exc.code} from {url}"
    except urllib.error.URLError as exc:
        return None, f"unreachable: {exc.reason}"
    except (TimeoutError, OSError) as exc:
        return None, f"network error: {exc}"
    except Exception as exc:  # noqa: BLE001 - defensive
        return None, f"unexpected error: {exc}"

    try:
        return json.loads(body.decode("utf-8")), None
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        return None, f"invalid JSON from {url}: {exc}"


def alert_id(alert: dict) -> str:
    """Compute a stable dedup id for an alert.

    The API alert shape has no explicit id, so we hash the identifying fields
    (kind + severity + message). Two structurally identical alerts therefore
    collapse to one notification across polls.
    """
    basis = "|".join([
        str(alert.get("kind", "")),
        str(alert.get("severity", "")),
        str(alert.get("message", "")),
    ])
    return hashlib.sha256(basis.encode("utf-8")).hexdigest()[:16]


def normalize_alert(alert: dict, feed_status: str, source: str) -> dict:
    """Convert a raw API alert into the normalized shape sinks expect."""
    return {
        "id": alert_id(alert),
        "kind": alert.get("kind", "unknown"),
        "severity": str(alert.get("severity", "info")).lower(),
        "message": alert.get("message", ""),
        "status": feed_status,
        "source": source,
    }


def meets_threshold(severity: str, threshold: str) -> bool:
    """Return True if ``severity`` is at or above the configured threshold."""
    sev_rank = SEVERITY_RANK.get(str(severity).lower(), 0)
    min_rank = SEVERITY_RANK.get(str(threshold).lower(), 0)
    return sev_rank >= min_rank


# ---------------------------------------------------------------------------
# Poll cycle
# ---------------------------------------------------------------------------

def poll_once(config: dict, sinks: list, seen: set) -> set:
    """Run a single poll-dispatch cycle. Returns the updated ``seen`` set.

    Pure with respect to its inputs: it builds and returns a *new* seen set
    rather than mutating the caller's, following the immutability rule.
    """
    base_url = config["api"]["base_url"].rstrip("/")
    threshold = config.get("severity_threshold", "info")
    include_readiness = bool(config.get("include_readiness", False))

    new_seen = set(seen)

    alerts_url = f"{base_url}/alerts"
    data, error = fetch_json(alerts_url)
    if error is not None:
        log(f"API unreachable / no alerts: {error}")
        # Optionally probe readiness too, but treat its failure the same way.
        if include_readiness:
            _poll_readiness(base_url)
        return new_seen

    raw_alerts = data.get("alerts") if isinstance(data, dict) else None
    feed_status = data.get("status", "unknown") if isinstance(data, dict) else "unknown"
    if not isinstance(raw_alerts, list):
        log("API responded but contained no alerts array; nothing to do")
        if include_readiness:
            _poll_readiness(base_url)
        return new_seen

    if not raw_alerts:
        log(f"API reachable, feed status '{feed_status}', no active alerts")
        if include_readiness:
            _poll_readiness(base_url)
        return new_seen

    dispatched = 0
    skipped_threshold = 0
    skipped_seen = 0
    for raw in raw_alerts:
        if not isinstance(raw, dict):
            continue
        alert = normalize_alert(raw, feed_status, base_url)
        if not meets_threshold(alert["severity"], threshold):
            skipped_threshold += 1
            continue
        if alert["id"] in new_seen:
            skipped_seen += 1
            continue
        new_seen.add(alert["id"])
        _dispatch(alert, sinks)
        dispatched += 1

    log(
        f"poll complete: {len(raw_alerts)} alerts in feed, "
        f"{dispatched} dispatched, {skipped_seen} already-seen, "
        f"{skipped_threshold} below threshold '{threshold}'"
    )
    if include_readiness:
        _poll_readiness(base_url)
    return new_seen


def _poll_readiness(base_url: str) -> None:
    """Probe /readiness for operator visibility. Failures are non-fatal."""
    data, error = fetch_json(f"{base_url}/readiness")
    if error is not None:
        log(f"readiness probe unavailable: {error}")
        return
    if isinstance(data, dict):
        status = data.get("status", "unknown")
        blocking = data.get("blocking", "?")
        log(f"readiness: status '{status}', {blocking} blocking check(s)")


def _dispatch(alert: dict, sinks: list) -> None:
    """Send one normalized alert to every configured sink."""
    log(
        f"new alert [{alert['severity']}] {alert['kind']}: {alert['message']} "
        f"(id={alert['id']})"
    )
    if not sinks:
        log("  no sinks configured/enabled; alert logged only")
        return
    for sink in sinks:
        result = sink.deliver(alert)
        state = "ok" if result.ok else "FAILED"
        log(f"  -> {result.sink}: {state} ({result.detail})")


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------

def parse_args(argv) -> argparse.Namespace:
    """Parse CLI arguments. Dry-run is the default; --live opts into network."""
    parser = argparse.ArgumentParser(
        description="Poll the Guardrail API and relay alerts to chat sinks.",
    )
    parser.add_argument(
        "--config",
        default=DEFAULT_CONFIG_PATH,
        help=f"path to config JSON (default: {DEFAULT_CONFIG_PATH})",
    )
    parser.add_argument(
        "--once",
        action="store_true",
        help="run a single poll cycle then exit (useful for testing)",
    )
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument(
        "--dry-run",
        dest="dry_run",
        action="store_true",
        default=True,
        help="print what would be sent; make no sink network calls (DEFAULT)",
    )
    mode.add_argument(
        "--live",
        dest="dry_run",
        action="store_false",
        help="actually deliver alerts to sinks (requires secrets in env)",
    )
    parser.add_argument(
        "--interval",
        type=float,
        default=None,
        help="override poll interval seconds (default: from config)",
    )
    return parser.parse_args(argv)


def run(argv=None) -> int:
    """Program entry point. Returns a process exit code.

    Always returns 0 for normal operation including a down API, so that
    --once --dry-run is a safe offline smoke test. Returns non-zero only for
    fatal configuration errors that an operator must fix.
    """
    args = parse_args(argv if argv is not None else sys.argv[1:])

    try:
        config = load_config(args.config)
    except (FileNotFoundError, ValueError) as exc:
        log(f"fatal: {exc}")
        return 2

    mode = "DRY-RUN (offline, no sink network calls)" if args.dry_run else "LIVE"
    log(f"Guardrail alert relay starting in {mode} mode")
    log(f"config: {args.config}")
    log(f"api base: {config['api']['base_url']}")

    sinks = build_sinks(config, dry_run=args.dry_run)
    if sinks:
        log(f"active sinks: {', '.join(s.name for s in sinks)}")
    else:
        log("no sinks enabled; alerts will be logged only")

    interval = args.interval
    if interval is None:
        interval = float(config.get("poll_interval_seconds", 60))

    seen: set = set()

    if args.once:
        seen = poll_once(config, sinks, seen)
        log("single poll complete; exiting (--once)")
        return 0

    log(f"entering poll loop every {interval:.0f}s (Ctrl+C to stop)")
    try:
        while True:
            seen = poll_once(config, sinks, seen)
            time.sleep(max(1.0, interval))
    except KeyboardInterrupt:
        log("interrupted; shutting down cleanly")
        return 0


if __name__ == "__main__":
    sys.exit(run())
