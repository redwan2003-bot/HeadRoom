//! Headroom Core: Pure domain models, memory state classifier, and configuration.
//!
//! # Safety Invariant
//! Strictly forbids any unsafe code and contains zero Windows API dependencies.

#![forbid(unsafe_code)]

pub mod classifier;
pub mod config;
pub mod error;
pub mod hysteresis;
pub mod types;

pub use classifier::classify;
pub use config::{Config, Thresholds, CURRENT_SCHEMA_VERSION};
pub use error::CoreError;
pub use hysteresis::HysteresisFilter;
pub use types::{
    CommitInfo, MemorySample, ProcessConsumer, StandbyBreakdown, StateDiagnosis, SystemState,
};
