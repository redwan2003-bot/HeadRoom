//! Typed errors for core domain operations.

use thiserror::Error;

/// Core domain errors.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Failure during configuration serialization or deserialization.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Missing or corrupted memory sample data.
    #[error("Invalid memory sample: {0}")]
    InvalidSample(String),

    /// Trace replay file could not be parsed.
    #[error("Trace replay error: {0}")]
    TraceReplayError(String),
}
