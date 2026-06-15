//! Logging, metrics, health checks, and alert descriptors.

pub mod alerts;
pub mod health;
pub mod logging;
pub mod metrics;
pub mod tracing_setup;

pub use alerts::{
    evaluate_all, evaluate_daily_trade, evaluate_data_age, evaluate_drawdown, evaluate_kill_switch,
    evaluate_reconciliation, evaluate_slippage, Alert, AlertInputs, AlertKind, AlertThresholds,
    Severity,
};
pub use health::{ComponentCheck, HealthStatus};
pub use metrics::{Metrics, MetricsSnapshot, METRICS_NAMESPACE};
