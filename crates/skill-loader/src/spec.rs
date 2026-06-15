//! Typed-ish view over a skill's `strategy_spec.yaml`.
//!
//! Strategy specs are large, evolving YAML documents. We parse the handful of
//! stable top-level identity fields into a typed struct and keep the full body
//! available as a dynamic [`serde_yaml::Value`] so callers can read deeper
//! sections (regime model, feature blend, sizing, risk policy) without this
//! crate having to track every field.

use crate::error::{Result, SkillLoaderError};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// The stable identity header parsed from a `strategy_spec.yaml`.
///
/// Only fields that are present and consistent across the shipped specs are
/// typed; everything else lives in [`SkillSpec::body`].
#[derive(Debug, Clone, Deserialize)]
pub struct SpecHeader {
    /// Semantic version of the spec document (e.g. `"2.0.0"`).
    #[serde(default)]
    pub spec_version: Option<String>,

    /// The strategy name as declared inside the spec (may differ from the
    /// catalog `id`; e.g. `regime-routed-bsc-alpha`).
    #[serde(default)]
    pub name: Option<String>,

    /// The hackathon track label, when present.
    #[serde(default)]
    pub track: Option<String>,

    /// Free-form description block.
    #[serde(default)]
    pub description: Option<String>,
}

/// A parsed strategy specification.
///
/// Combines the typed [`SpecHeader`] with the complete dynamic document so no
/// information is lost.
#[derive(Debug, Clone)]
pub struct SkillSpec {
    /// Typed identity fields.
    pub header: SpecHeader,

    /// The complete spec document as a dynamic YAML value.
    pub body: serde_yaml::Value,
}

impl SkillSpec {
    /// Load and parse a `strategy_spec.yaml` from `path`.
    ///
    /// Returns [`SkillLoaderError::MissingPath`] if the file does not exist,
    /// and never panics.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(SkillLoaderError::MissingPath(path.to_path_buf()));
        }

        let raw = fs::read_to_string(path).map_err(|source| SkillLoaderError::Io {
            path: path.to_path_buf(),
            source,
        })?;

        Self::from_str(&raw, path)
    }

    /// Parse a spec from an in-memory YAML string. `origin` is used only for
    /// error reporting.
    pub fn from_str(raw: &str, origin: &Path) -> Result<Self> {
        let body: serde_yaml::Value =
            serde_yaml::from_str(raw).map_err(|source| SkillLoaderError::Yaml {
                path: origin.to_path_buf(),
                source,
            })?;

        let header: SpecHeader =
            serde_yaml::from_str(raw).map_err(|source| SkillLoaderError::Yaml {
                path: origin.to_path_buf(),
                source,
            })?;

        Ok(Self { header, body })
    }

    /// Convenience accessor for a top-level mapping key from the dynamic body.
    pub fn section(&self, key: &str) -> Option<&serde_yaml::Value> {
        self.body.get(key)
    }
}
