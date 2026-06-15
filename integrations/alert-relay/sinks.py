"""Alert sink abstractions for the Guardrail alert relay.

Each sink knows how to (1) format a normalized alert into a chat-friendly
message and (2) deliver it to its destination. Every sink supports a
``dry_run`` path that prints what *would* be sent instead of opening a
network connection, so the relay runs fully offline with no secrets.

The relay (``relay.py``) owns polling and dedup; this module owns delivery.
All network access goes through the Python standard library (``urllib``)
so there are zero third-party dependencies.

A "normalized alert" is a plain dict with at least these keys (see
``relay.normalize_alert``):

    {
        "id":       str,   # stable dedup key (hash of kind+severity+message)
        "kind":     str,   # e.g. "drawdown_hard", "kill_switch"
        "severity": str,   # "info" | "warning" | "critical"
        "message":  str,   # human-readable detail
        "status":   str,   # overall feed status at observation time
        "source":   str,   # api base url the alert came from
    }
"""

from __future__ import annotations

import email.message
import email.utils
import json
import smtplib
import urllib.error
import urllib.request

# Network timeout (seconds) applied to every outbound request. Kept small so a
# hung sink never stalls the poll loop.
DEFAULT_TIMEOUT = 10

# Emoji-free severity markers; rules forbid emojis in assistant output and we
# keep sink payloads plain so they render everywhere (Telegram, Discord, logs).
SEVERITY_MARKERS = {
    "critical": "[CRITICAL]",
    "warning": "[WARNING]",
    "info": "[INFO]",
}


def severity_marker(severity: str) -> str:
    """Return a stable textual marker for a severity, defaulting safely."""
    return SEVERITY_MARKERS.get((severity or "").lower(), "[ALERT]")


def format_alert_text(alert: dict) -> str:
    """Render a normalized alert into a single plain-text block.

    Shared by all sinks so the wording is consistent across destinations.
    """
    marker = severity_marker(alert.get("severity", ""))
    kind = alert.get("kind", "unknown")
    message = alert.get("message", "")
    source = alert.get("source", "")
    feed_status = alert.get("status", "")
    lines = [
        f"{marker} Guardrail alert: {kind}",
        message,
    ]
    if feed_status:
        lines.append(f"feed status: {feed_status}")
    if source:
        lines.append(f"source: {source}")
    return "\n".join(line for line in lines if line)


class DeliveryResult:
    """Outcome of a single delivery attempt.

    Immutable value object: construct a new one rather than mutating fields.
    """

    __slots__ = ("sink", "ok", "detail", "dry_run")

    def __init__(self, sink: str, ok: bool, detail: str, dry_run: bool) -> None:
        self.sink = sink
        self.ok = ok
        self.detail = detail
        self.dry_run = dry_run

    def __repr__(self) -> str:  # pragma: no cover - debugging aid
        mode = "dry-run" if self.dry_run else "live"
        state = "ok" if self.ok else "FAILED"
        return f"<DeliveryResult {self.sink} {mode} {state}: {self.detail}>"


def _post_json(url: str, payload: dict, timeout: int) -> DeliveryResult:
    """POST a JSON body and classify the result. Never raises.

    Returns a DeliveryResult describing success or the failure reason. All
    error modes (DNS, connection refused, timeout, HTTP error, bad URL) are
    caught so a single bad sink can never crash the relay.
    """
    data = json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(
        url,
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            code = getattr(response, "status", response.getcode())
            return DeliveryResult("", True, f"http {code}", False)
    except urllib.error.HTTPError as exc:
        return DeliveryResult("", False, f"http error {exc.code}", False)
    except urllib.error.URLError as exc:
        return DeliveryResult("", False, f"url error: {exc.reason}", False)
    except (TimeoutError, OSError) as exc:
        return DeliveryResult("", False, f"network error: {exc}", False)
    except Exception as exc:  # noqa: BLE001 - defensive: never crash the loop
        return DeliveryResult("", False, f"unexpected error: {exc}", False)


class Sink:
    """Base sink: formats and delivers a single alert.

    Subclasses implement ``_deliver_live``. The base class owns the dry-run
    path and the safety net so subclasses stay tiny.
    """

    name = "sink"

    def __init__(self, *, dry_run: bool, timeout: int = DEFAULT_TIMEOUT) -> None:
        self.dry_run = dry_run
        self.timeout = timeout

    def deliver(self, alert: dict) -> DeliveryResult:
        """Deliver one alert, honoring dry-run and never raising."""
        if self.dry_run:
            preview = format_alert_text(alert)
            print(f"  [{self.name}] DRY-RUN would send:")
            for line in preview.splitlines():
                print(f"    | {line}")
            return DeliveryResult(self.name, True, "dry-run preview", True)
        try:
            result = self._deliver_live(alert)
        except Exception as exc:  # noqa: BLE001 - defensive guard
            result = DeliveryResult(self.name, False, f"unexpected error: {exc}", False)
        result.sink = self.name
        return result

    def _deliver_live(self, alert: dict) -> DeliveryResult:  # pragma: no cover
        raise NotImplementedError


class TelegramSink(Sink):
    """Deliver alerts via the Telegram Bot API ``sendMessage`` endpoint.

    The bot token is supplied via config/env (never hardcoded). ``chat_id``
    selects the destination chat or channel.
    """

    name = "telegram"

    def __init__(self, *, token: str, chat_id: str, dry_run: bool,
                 timeout: int = DEFAULT_TIMEOUT,
                 api_base: str = "https://api.telegram.org") -> None:
        super().__init__(dry_run=dry_run, timeout=timeout)
        self.token = token
        self.chat_id = chat_id
        self.api_base = api_base.rstrip("/")

    def _deliver_live(self, alert: dict) -> DeliveryResult:
        if not self.token or not self.chat_id:
            return DeliveryResult(
                self.name, False, "missing token or chat_id", False
            )
        url = f"{self.api_base}/bot{self.token}/sendMessage"
        payload = {
            "chat_id": self.chat_id,
            "text": format_alert_text(alert),
            "disable_web_page_preview": True,
        }
        return _post_json(url, payload, self.timeout)


class DiscordSink(Sink):
    """Deliver alerts to a Discord channel via an incoming webhook URL."""

    name = "discord"

    def __init__(self, *, webhook_url: str, dry_run: bool,
                 timeout: int = DEFAULT_TIMEOUT) -> None:
        super().__init__(dry_run=dry_run, timeout=timeout)
        self.webhook_url = webhook_url

    def _deliver_live(self, alert: dict) -> DeliveryResult:
        if not self.webhook_url:
            return DeliveryResult(self.name, False, "missing webhook_url", False)
        payload = {"content": format_alert_text(alert)}
        return _post_json(self.webhook_url, payload, self.timeout)


class WebhookSink(Sink):
    """Deliver alerts to a generic HTTP endpoint as a structured JSON body.

    Unlike the chat sinks, this forwards the full normalized alert plus a
    rendered ``text`` field so downstream systems can route on structure.
    """

    name = "webhook"

    def __init__(self, *, url: str, dry_run: bool,
                 timeout: int = DEFAULT_TIMEOUT) -> None:
        super().__init__(dry_run=dry_run, timeout=timeout)
        self.url = url

    def _deliver_live(self, alert: dict) -> DeliveryResult:
        if not self.url:
            return DeliveryResult(self.name, False, "missing url", False)
        payload = {
            "source": "guardrail-alert-relay",
            "text": format_alert_text(alert),
            "alert": alert,
        }
        return _post_json(self.url, payload, self.timeout)


class SlackSink(Sink):
    """Deliver alerts to a Slack channel via an incoming-webhook URL.

    The webhook URL (which embeds the secret token) is supplied via config/env
    and never hardcoded. Slack's incoming-webhook API accepts a simple JSON
    body ``{"text": "..."}``; we send the shared plain-text rendering so the
    message reads identically across every sink.
    """

    name = "slack"

    def __init__(self, *, webhook_url: str, dry_run: bool,
                 timeout: int = DEFAULT_TIMEOUT) -> None:
        super().__init__(dry_run=dry_run, timeout=timeout)
        self.webhook_url = webhook_url

    def _deliver_live(self, alert: dict) -> DeliveryResult:
        if not self.webhook_url:
            return DeliveryResult(self.name, False, "missing webhook_url", False)
        payload = {"text": format_alert_text(alert)}
        return _post_json(self.webhook_url, payload, self.timeout)


class EmailSink(Sink):
    """Deliver alerts as email via SMTP using only the standard library.

    Uses ``smtplib`` + ``email.message`` so there are zero third-party
    dependencies. The dry-run path renders and prints the fully composed
    message (headers + body) without opening any socket; a live send only
    happens when ``dry_run`` is False, and even then every network operation
    is guarded so a misconfigured or unreachable mail server can never crash
    the relay.

    All credentials (host, username, password) come from env-referenced config
    resolved by the relay; nothing is hardcoded. STARTTLS is used when
    ``use_tls`` is set (the default) and the server advertises support.
    """

    name = "email"

    def __init__(self, *, host: str, port: int, sender: str, recipients: list,
                 username: str, password: str, use_tls: bool, dry_run: bool,
                 timeout: int = DEFAULT_TIMEOUT,
                 subject_prefix: str = "[Guardrail]") -> None:
        super().__init__(dry_run=dry_run, timeout=timeout)
        self.host = host
        self.port = port
        self.sender = sender
        self.recipients = recipients
        self.username = username
        self.password = password
        self.use_tls = use_tls
        self.subject_prefix = subject_prefix

    def _subject(self, alert: dict) -> str:
        """Compose a concise, scannable subject line."""
        marker = severity_marker(alert.get("severity", ""))
        kind = alert.get("kind", "unknown")
        return f"{self.subject_prefix} {marker} {kind}".strip()

    def _build_message(self, alert: dict) -> email.message.EmailMessage:
        """Build a fully-formed RFC 5322 message for this alert."""
        msg = email.message.EmailMessage()
        msg["Subject"] = self._subject(alert)
        if self.sender:
            msg["From"] = self.sender
        if self.recipients:
            msg["To"] = ", ".join(self.recipients)
        msg["Date"] = email.utils.formatdate(localtime=True)
        msg["Message-ID"] = email.utils.make_msgid(domain="guardrail.local")
        msg.set_content(format_alert_text(alert))
        return msg

    def deliver(self, alert: dict) -> DeliveryResult:
        """Deliver one alert via email, honoring dry-run and never raising.

        Overrides the base dry-run preview so the operator sees the exact
        composed message (subject + recipients + body) rather than the generic
        text-only preview shared by the chat sinks.
        """
        if self.dry_run:
            message = self._build_message(alert)
            print(f"  [{self.name}] DRY-RUN would send (no SMTP connection):")
            for line in str(message).splitlines():
                print(f"    | {line}")
            return DeliveryResult(self.name, True, "dry-run preview", True)
        try:
            result = self._deliver_live(alert)
        except Exception as exc:  # noqa: BLE001 - defensive guard
            result = DeliveryResult(self.name, False, f"unexpected error: {exc}", False)
        result.sink = self.name
        return result

    def _deliver_live(self, alert: dict) -> DeliveryResult:
        if not self.host or not self.sender or not self.recipients:
            return DeliveryResult(
                self.name, False, "missing host, sender, or recipients", False
            )
        message = self._build_message(alert)
        try:
            with smtplib.SMTP(self.host, self.port, timeout=self.timeout) as server:
                if self.use_tls:
                    server.starttls()
                if self.username and self.password:
                    server.login(self.username, self.password)
                server.send_message(message)
            return DeliveryResult(self.name, True, f"sent to {len(self.recipients)} recipient(s)", False)
        except smtplib.SMTPException as exc:
            return DeliveryResult(self.name, False, f"smtp error: {exc}", False)
        except (TimeoutError, OSError) as exc:
            return DeliveryResult(self.name, False, f"network error: {exc}", False)
        except Exception as exc:  # noqa: BLE001 - defensive: never crash the loop
            return DeliveryResult(self.name, False, f"unexpected error: {exc}", False)


def _as_recipient_list(raw) -> list:
    """Normalize a recipients config value into a list of address strings.

    Accepts either a JSON array or a comma/semicolon-separated string so the
    config is forgiving without trusting the shape blindly.
    """
    if isinstance(raw, list):
        return [str(item).strip() for item in raw if str(item).strip()]
    if isinstance(raw, str):
        parts = raw.replace(";", ",").split(",")
        return [part.strip() for part in parts if part.strip()]
    return []


def _as_bool(raw, default: bool = True) -> bool:
    """Coerce a config value into a bool, tolerating strings like 'false'."""
    if isinstance(raw, bool):
        return raw
    if isinstance(raw, str):
        return raw.strip().lower() in ("1", "true", "yes", "on")
    if raw is None:
        return default
    return bool(raw)


def _as_int(raw, default: int) -> int:
    """Coerce a config value into an int, falling back on bad input."""
    try:
        return int(raw)
    except (TypeError, ValueError):
        return default


def build_sink(kind: str, settings: dict, *, dry_run: bool,
               timeout: int = DEFAULT_TIMEOUT) -> Sink:
    """Construct a sink from a config entry.

    ``settings`` is the per-sink config dict. Secrets are resolved by the
    caller (relay.py) from env vars before this is called. Raises
    ``ValueError`` for an unknown sink kind so misconfiguration fails fast.
    """
    kind = (kind or "").lower()
    if kind == "telegram":
        return TelegramSink(
            token=settings.get("token", ""),
            chat_id=settings.get("chat_id", ""),
            dry_run=dry_run,
            timeout=timeout,
            api_base=settings.get("api_base", "https://api.telegram.org"),
        )
    if kind == "discord":
        return DiscordSink(
            webhook_url=settings.get("webhook_url", ""),
            dry_run=dry_run,
            timeout=timeout,
        )
    if kind == "webhook":
        return WebhookSink(
            url=settings.get("url", ""),
            dry_run=dry_run,
            timeout=timeout,
        )
    if kind == "slack":
        return SlackSink(
            webhook_url=settings.get("webhook_url", ""),
            dry_run=dry_run,
            timeout=timeout,
        )
    if kind == "email":
        return EmailSink(
            host=settings.get("host", ""),
            port=_as_int(settings.get("port", 587), 587),
            sender=settings.get("sender", ""),
            recipients=_as_recipient_list(settings.get("recipients", [])),
            username=settings.get("username", ""),
            password=settings.get("password", ""),
            use_tls=_as_bool(settings.get("use_tls", True), default=True),
            dry_run=dry_run,
            timeout=timeout,
            subject_prefix=settings.get("subject_prefix", "[Guardrail]"),
        )
    raise ValueError(f"unknown sink kind: {kind!r}")
