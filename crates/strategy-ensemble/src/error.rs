//! Error type for the ensemble meta-allocator.

use thiserror::Error;

/// Errors that can arise while loading or parsing the ensemble config.
///
/// Note: the blend math itself ([`crate::blend_targets`]) never errors — it
/// returns a typed result with a `reason` instead of panicking or failing.
#[derive(Debug, Error)]
pub enum EnsembleError {
    /// The config file could not be read from disk.
    #[error("failed to read ensemble config at '{path}': {source}")]
    Read {
        /// The path that failed to read.
        path: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// The config JSON could not be parsed.
    #[error("failed to parse ensemble config: {0}")]
    Parse(#[source] serde_json::Error),
}
