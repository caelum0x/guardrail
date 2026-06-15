//! Reusable notification primitives for guardrail services.
//!
//! Defines a small [`Notification`] value type and a [`Sink`] trait that abstracts
//! over delivery targets. Concrete sinks ([`ConsoleSink`], [`FileSink`],
//! [`WebhookSink`]) cover local logging, durable append-only files, and outbound
//! HTTP webhooks respectively.

use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Maximum time to wait for a webhook endpoint before giving up.
const WEBHOOK_TIMEOUT: Duration = Duration::from_secs(5);

/// A single notification describing an event worth surfacing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    /// Origin of the notification (e.g. a service name).
    pub source: String,
    /// Severity label (e.g. `WARNING`, `CRITICAL`).
    pub severity: String,
    /// Human-readable description of the event.
    pub message: String,
}

impl Notification {
    /// Build a new notification from any displayable parts.
    pub fn new(
        source: impl Into<String>,
        severity: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            severity: severity.into(),
            message: message.into(),
        }
    }
}

/// A delivery target for a batch of [`Notification`]s.
///
/// Implementations must be safe to share across threads so a single sink can be
/// reused by concurrent producers.
#[async_trait]
pub trait Sink: Send + Sync {
    /// Deliver the given notifications.
    ///
    /// Implementations should treat an empty slice as a no-op and return `Ok`.
    async fn deliver(&self, notes: &[Notification]) -> anyhow::Result<()>;
}

/// A sink that logs notifications via `tracing`.
#[derive(Debug, Default, Clone)]
pub struct ConsoleSink;

impl ConsoleSink {
    /// Create a new console sink.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Sink for ConsoleSink {
    async fn deliver(&self, notes: &[Notification]) -> anyhow::Result<()> {
        for note in notes {
            tracing::info!(
                source = %note.source,
                severity = %note.severity,
                message = %note.message,
                "notification"
            );
        }
        Ok(())
    }
}

/// A sink that appends each notification as a JSON line to a file.
#[derive(Debug, Clone)]
pub struct FileSink {
    /// Path of the append-only JSON Lines file.
    pub path: PathBuf,
}

impl FileSink {
    /// Create a new file sink targeting `path`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait]
impl Sink for FileSink {
    async fn deliver(&self, notes: &[Notification]) -> anyhow::Result<()> {
        if notes.is_empty() {
            return Ok(());
        }
        let mut buffer = String::new();
        for note in notes {
            let line = serde_json::to_string(note)
                .map_err(|e| anyhow::anyhow!("failed to serialize notification: {e}"))?;
            buffer.push_str(&line);
            buffer.push('\n');
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| {
                anyhow::anyhow!("failed to open {} for append: {e}", self.path.display())
            })?;
        file.write_all(buffer.as_bytes())
            .map_err(|e| anyhow::anyhow!("failed to write to {}: {e}", self.path.display()))?;
        Ok(())
    }
}

/// A sink that POSTs notifications as JSON to a webhook URL.
///
/// The request body has the shape:
/// ```json
/// { "source": "...", "count": 2, "notes": [ { "source": "...", "severity": "...", "message": "..." } ] }
/// ```
#[derive(Debug, Clone)]
pub struct WebhookSink {
    /// Destination webhook URL.
    pub url: String,
    /// Pre-built HTTP client used for delivery.
    pub client: reqwest::Client,
}

impl WebhookSink {
    /// Create a new webhook sink targeting `url`, building a client with a 5s timeout.
    pub fn new(url: impl Into<String>) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(WEBHOOK_TIMEOUT)
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build webhook client: {e}"))?;
        Ok(Self {
            url: url.into(),
            client,
        })
    }

    /// Create a webhook sink from an existing client (e.g. to share a connection pool).
    pub fn with_client(url: impl Into<String>, client: reqwest::Client) -> Self {
        Self {
            url: url.into(),
            client,
        }
    }
}

#[async_trait]
impl Sink for WebhookSink {
    async fn deliver(&self, notes: &[Notification]) -> anyhow::Result<()> {
        if notes.is_empty() {
            return Ok(());
        }
        let source = notes
            .first()
            .map(|n| n.source.as_str())
            .unwrap_or("notifier");
        let payload = json!({
            "source": source,
            "count": notes.len(),
            "notes": notes,
        });
        let response = self
            .client
            .post(&self.url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("failed to POST notifications to webhook: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "webhook returned non-success status: {status}"
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn console_sink_returns_ok() {
        let sink = ConsoleSink::new();
        let notes = vec![Notification::new("test", "WARNING", "hello")];
        let result = sink.deliver(&notes).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn file_sink_writes_a_line() {
        let mut path = std::env::temp_dir();
        path.push(format!("notifier_test_{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let sink = FileSink::new(&path);
        let notes = vec![Notification::new(
            "monitor",
            "CRITICAL",
            "kill switch engaged",
        )];
        sink.deliver(&notes)
            .await
            .expect("file sink delivery should succeed");

        let contents = std::fs::read_to_string(&path).expect("file should exist");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 1);

        let parsed: Notification =
            serde_json::from_str(lines[0]).expect("line should be valid notification JSON");
        assert_eq!(parsed.source, "monitor");
        assert_eq!(parsed.severity, "CRITICAL");
        assert_eq!(parsed.message, "kill switch engaged");

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn file_sink_empty_is_noop() {
        let mut path = std::env::temp_dir();
        path.push(format!("notifier_empty_{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let sink = FileSink::new(&path);
        sink.deliver(&[]).await.expect("empty delivery is ok");
        assert!(!path.exists(), "no file should be created for empty batch");
    }
}
