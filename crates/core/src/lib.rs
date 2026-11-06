//! Headroom Core: Pure domain models, memory state classifier, and configuration.
//!
//! # Safety Invariant
//! Strictly forbids any unsafe code and contains zero Windows API dependencies.

#![forbid(unsafe_code)]

pub mod classifier;
pub mod config;
pub mod error;
pub mod history;
pub mod hysteresis;
pub mod i18n;
pub mod policy;
pub mod types;

pub use classifier::classify;
pub use config::{
    get_config_path, is_portable_mode, Config, OperatingMode, Thresholds, CURRENT_SCHEMA_VERSION,
};
pub use error::CoreError;
pub use history::{HistoryPoint, MemoryHistory};
pub use hysteresis::HysteresisFilter;
pub use i18n::{tr, Language, TextId};
pub use policy::{decide, PolicyDecision, PolicyState};
pub use types::{
    CommitInfo, MemorySample, ProcessConsumer, StandbyBreakdown, StateDiagnosis, SystemState,
};
