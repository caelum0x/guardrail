//! A thread-safe, in-process metrics registry.
//!
//! Counters are monotonically increasing `u64` values; gauges are arbitrary
//! `f64` values that may move up or down. The registry uses only `std` sync
//! primitives (a `Mutex` per metric family) and carries no external metrics
//! dependency.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::Serialize;

/// The namespace prefix applied to exported metric names.
pub const METRICS_NAMESPACE: &str = "guardrail_alpha";

/// A point-in-time, serializable view of the registry.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MetricsSnapshot {
    /// Namespace the metrics belong to.
    pub namespace: String,
    /// Counter name to current value.
    pub counters: BTreeMap<String, u64>,
    /// Gauge name to current value.
    pub gauges: BTreeMap<String, f64>,
}

/// A thread-safe in-process registry of counters and gauges.
///
/// `Metrics` is `Send + Sync` and is intended to be shared behind an `Arc`.
/// All mutating methods take `&self`.
#[derive(Debug)]
pub struct Metrics {
    namespace: String,
    counters: Mutex<BTreeMap<String, u64>>,
    gauges: Mutex<BTreeMap<String, f64>>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new(METRICS_NAMESPACE)
    }
}

impl Metrics {
    /// Create an empty registry under the given namespace.
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            counters: Mutex::new(BTreeMap::new()),
            gauges: Mutex::new(BTreeMap::new()),
        }
    }

    /// Increment a counter by one, creating it at zero if absent.
    pub fn incr(&self, name: &str) {
        self.add(name, 1);
    }

    /// Add `n` to a counter, creating it at zero if absent.
    ///
    /// A poisoned lock is recovered from rather than propagated, since a
    /// metrics write must never bring down a trading process.
    pub fn add(&self, name: &str, n: u64) {
        let mut counters = match self.counters.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let entry = counters.entry(name.to_string()).or_insert(0);
        *entry = entry.saturating_add(n);
    }

    /// Set a gauge to an absolute value, creating it if absent.
    pub fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = match self.gauges.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        gauges.insert(name.to_string(), value);
    }

    /// Read the current value of a counter, if it exists.
    pub fn counter(&self, name: &str) -> Option<u64> {
        let counters = match self.counters.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        counters.get(name).copied()
    }

    /// Read the current value of a gauge, if it exists.
    pub fn gauge(&self, name: &str) -> Option<f64> {
        let gauges = match self.gauges.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        gauges.get(name).copied()
    }

    /// Capture a consistent, serializable copy of all metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let counters = match self.counters.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        let gauges = match self.gauges.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        MetricsSnapshot {
            namespace: self.namespace.clone(),
            counters,
            gauges,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incr_creates_and_increments_counter() {
        let m = Metrics::new("test");
        m.incr("orders");
        m.incr("orders");
        assert_eq!(m.counter("orders"), Some(2));
    }

    #[test]
    fn add_accumulates_counter() {
        let m = Metrics::new("test");
        m.add("fills", 5);
        m.add("fills", 3);
        assert_eq!(m.counter("fills"), Some(8));
    }

    #[test]
    fn set_gauge_overwrites_value() {
        let m = Metrics::new("test");
        m.set_gauge("equity", 100.0);
        m.set_gauge("equity", 95.5);
        assert_eq!(m.gauge("equity"), Some(95.5));
    }

    #[test]
    fn unknown_metric_returns_none() {
        let m = Metrics::new("test");
        assert_eq!(m.counter("missing"), None);
        assert_eq!(m.gauge("missing"), None);
    }

    #[test]
    fn snapshot_captures_all_metrics() {
        let m = Metrics::new("ns");
        m.add("a", 2);
        m.set_gauge("g", 1.5);
        let snap = m.snapshot();
        assert_eq!(snap.namespace, "ns");
        assert_eq!(snap.counters.get("a"), Some(&2));
        assert_eq!(snap.gauges.get("g"), Some(&1.5));
    }

    #[test]
    fn snapshot_is_json_serializable() {
        let m = Metrics::new("ns");
        m.incr("a");
        m.set_gauge("g", 2.0);
        let json = serde_json::to_string(&m.snapshot()).expect("serialize");
        assert!(json.contains("\"a\":1"));
        assert!(json.contains("\"g\":2.0"));
    }

    #[test]
    fn shared_registry_is_thread_safe() {
        use std::sync::Arc;
        use std::thread;

        let m = Arc::new(Metrics::new("test"));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let m = Arc::clone(&m);
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    m.incr("ticks");
                }
            }));
        }
        for h in handles {
            h.join().expect("thread join");
        }
        assert_eq!(m.counter("ticks"), Some(8000));
    }
}
