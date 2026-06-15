//! Typed errors for the skill loader.
//!
//! The loader never panics on missing or malformed files: every fallible
//! operation returns a [`Result`] carrying one of these variants.

use std::path::PathBuf;
use thiserror::Error;

/// Crate-local result alias.
pub type Result<T> = std::result::Result<T, SkillLoaderError>;

/// Errors raised while loading or validating the on-disk skill registry.
#[derive(Debug, Error)]
pub enum SkillLoaderError {
    /// The catalog index file (`skills/INDEX.json`) was not found on disk.
    #[error("skill index not found at {0}")]
    IndexNotFound(PathBuf),

    /// A path referenced by the catalog (a skill directory or spec file) is
    /// missing from disk.
    #[error("path declared in catalog does not exist: {0}")]
    MissingPath(PathBuf),

    /// No skill in the catalog matched the requested id.
    #[error("unknown skill id: {0}")]
    UnknownSkill(String),

    /// An underlying I/O failure (permissions, unreadable file, etc.).
    #[error("io error reading {path}: {source}")]
    Io {
        /// The path that triggered the failure.
        path: PathBuf,
        /// The underlying OS error.
        #[source]
        source: std::io::Error,
    },

    /// The JSON index could not be parsed into the typed catalog.
    #[error("failed to parse skill index {path}: {source}")]
    Json {
        /// The file that failed to parse.
        path: PathBuf,
        /// The underlying serde_json error.
        #[source]
        source: serde_json::Error,
    },

    /// A `strategy_spec.yaml` could not be parsed.
    #[error("failed to parse strategy spec {path}: {source}")]
    Yaml {
        /// The file that failed to parse.
        path: PathBuf,
        /// The underlying serde_yaml error.
        #[source]
        source: serde_yaml::Error,
    },
}
