//! Component-level health checks and overall status aggregation.
//!
//! [`HealthStatus`] is a serializable snapshot of named component checks. The
//! overall status is healthy only when every component is healthy.

use serde::{Deserialize, Serialize};

/// The result of a single named component check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentCheck {
    /// Component name (for example `data_fresh`).
    pub name: String,
    /// Whether the component is healthy.
    pub healthy: bool,
    /// Human-readable detail.
    pub detail: String,
}

impl ComponentCheck {
    /// Build a check from a boolean and a detail message.
    pub fn new(name: impl Into<String>, healthy: bool, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            healthy,
            detail: detail.into(),
        }
    }
}

/// Aggregated health across all monitored components.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HealthStatus {
    /// Individual component checks in declaration order.
    pub components: Vec<ComponentCheck>,
}

impl HealthStatus {
    /// Build a status from the three core trading-loop component checks.
    pub fn from_components(data_fresh: bool, executor_ok: bool, within_drawdown: bool) -> Self {
        Self {
            components: vec![
                ComponentCheck::new(
                    "data_fresh",
                    data_fresh,
                    if data_fresh {
                        "market data is fresh"
                    } else {
                        "market data is stale"
                    },
                ),
                ComponentCheck::new(
                    "executor_ok",
                    executor_ok,
                    if executor_ok {
                        "executor reachable"
                    } else {
                        "executor unavailable"
                    },
                ),
                ComponentCheck::new(
                    "within_drawdown",
                    within_drawdown,
                    if within_drawdown {
                        "drawdown within limits"
                    } else {
                        "drawdown limit breached"
                    },
                ),
            ],
        }
    }

    /// A fully healthy status with no failing components.
    pub fn healthy() -> Self {
        Self::from_components(true, true, true)
    }

    /// Add or replace a named component check, returning a new status.
    ///
    /// Immutable: the receiver is consumed and a new value is returned rather
    /// than mutating in place.
    pub fn with_component(
        self,
        name: impl Into<String>,
        healthy: bool,
        detail: impl Into<String>,
    ) -> Self {
        let name = name.into();
        let check = ComponentCheck::new(name.clone(), healthy, detail);
        let mut components: Vec<ComponentCheck> = self
            .components
            .into_iter()
            .filter(|c| c.name != name)
            .collect();
        components.push(check);
        Self { components }
    }

    /// Whether every component is healthy. An empty status is considered ok.
    pub fn ok(&self) -> bool {
        self.components.iter().all(|c| c.healthy)
    }

    /// The names of any failing components, in declaration order.
    pub fn failing(&self) -> Vec<&str> {
        self.components
            .iter()
            .filter(|c| !c.healthy)
            .map(|c| c.name.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_healthy_components_are_ok() {
        let status = HealthStatus::from_components(true, true, true);
        assert!(status.ok());
        assert!(status.failing().is_empty());
        assert_eq!(status.components.len(), 3);
    }

    #[test]
    fn any_failing_component_makes_overall_unhealthy() {
        let status = HealthStatus::from_components(true, false, true);
        assert!(!status.ok());
        assert_eq!(status.failing(), vec!["executor_ok"]);
    }

    #[test]
    fn multiple_failures_are_all_reported() {
        let status = HealthStatus::from_components(false, true, false);
        assert!(!status.ok());
        assert_eq!(status.failing(), vec!["data_fresh", "within_drawdown"]);
    }

    #[test]
    fn empty_status_is_ok() {
        let status = HealthStatus::default();
        assert!(status.ok());
    }

    #[test]
    fn with_component_replaces_existing_check() {
        let status = HealthStatus::healthy().with_component("data_fresh", false, "feed dropped");
        assert!(!status.ok());
        assert_eq!(status.failing(), vec!["data_fresh"]);
        // Replacing should not duplicate the component.
        let count = status
            .components
            .iter()
            .filter(|c| c.name == "data_fresh")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn with_component_adds_new_check() {
        let status = HealthStatus::healthy().with_component("broker_auth", true, "token valid");
        assert!(status.ok());
        assert!(status.components.iter().any(|c| c.name == "broker_auth"));
    }

    #[test]
    fn status_is_json_serializable() {
        let status = HealthStatus::from_components(true, false, true);
        let json = serde_json::to_string(&status).expect("serialize");
        assert!(json.contains("executor_ok"));
        assert!(json.contains("data_fresh"));
    }
}
