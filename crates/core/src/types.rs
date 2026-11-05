//! Core memory data types and state definitions.
//!
//! Enforces `#![forbid(unsafe_code)]` with zero OS-specific dependencies.

use serde::{Deserialize, Serialize};

/// Breakdown of the Windows Standby Cache list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StandbyBreakdown {
    /// Memory\Standby Cache Reserve Bytes
    pub reserve_bytes: u64,
    /// Memory\Standby Cache Normal Priority Bytes
    pub normal_bytes: u64,
    /// Memory\Standby Cache Core Bytes
    pub core_bytes: u64,
}

impl StandbyBreakdown {
    /// Calculates total standby bytes across all priority tiers.
    pub fn total(&self) -> u64 {
        self.reserve_bytes
            .saturating_add(self.normal_bytes)
            .saturating_add(self.core_bytes)
    }
}

/// System commit charge information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CommitInfo {
    /// Current committed memory in bytes.
    pub total_bytes: u64,
    /// Current commit limit in bytes.
    pub limit_bytes: u64,
    /// Peak commit recorded since boot in bytes.
    pub peak_bytes: u64,
}

impl CommitInfo {
    /// Returns current commit ratio as a float in [0.0, 1.0].
    pub fn ratio(&self) -> f64 {
        if self.limit_bytes == 0 {
            0.0
        } else {
            (self.total_bytes as f64) / (self.limit_bytes as f64)
        }
    }

    /// Returns current commit usage as a percentage [0, 100].
    pub fn percent(&self) -> u8 {
        (self.ratio() * 100.0).clamp(0.0, 100.0) as u8
    }
}

/// A running process consuming memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessConsumer {
    /// Process identifier.
    pub pid: u32,
    /// Process image name (e.g. "chrome.exe").
    pub name: String,
    /// Private committed bytes.
    pub private_bytes: u64,
    /// Physical working set bytes.
    pub working_set_bytes: u64,
    /// Whether private memory grew in recent samples.
    pub is_growing: bool,
}

/// A snapshot of system memory state at a point in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemorySample {
    /// Unix timestamp in seconds.
    pub timestamp_utc: u64,
    /// Total installed physical RAM in bytes.
    pub physical_total_bytes: u64,
    /// Total available physical memory (free + zeroed + standby) in bytes.
    pub available_bytes: u64,
    /// Physical memory in active use by processes and kernel in bytes.
    pub in_use_bytes: u64,
    /// Memory\Modified Page List Bytes.
    pub modified_bytes: u64,
    /// Standby cache breakdown.
    pub standby: StandbyBreakdown,
    /// Memory\Free & Zero Page List Bytes.
    pub free_zero_bytes: u64,
    /// System commit charge metrics.
    pub commit: CommitInfo,
    /// Rate of pages read from disk to resolve hard page faults (Pages Input/sec).
    pub hard_faults_per_sec: f64,
    /// Top process memory consumers sorted descending by private bytes.
    pub top_consumers: Vec<ProcessConsumer>,
}

impl MemorySample {
    /// Ratio of available memory to total physical memory [0.0, 1.0].
    pub fn available_ratio(&self) -> f64 {
        if self.physical_total_bytes == 0 {
            0.0
        } else {
            (self.available_bytes as f64) / (self.physical_total_bytes as f64)
        }
    }

    /// Ratio of total standby cache to total physical memory [0.0, 1.0].
    pub fn standby_ratio(&self) -> f64 {
        if self.physical_total_bytes == 0 {
            0.0
        } else {
            (self.standby.total() as f64) / (self.physical_total_bytes as f64)
        }
    }

    /// Ratio of free & zeroed pages to total physical memory [0.0, 1.0].
    pub fn free_zero_ratio(&self) -> f64 {
        if self.physical_total_bytes == 0 {
            0.0
        } else {
            (self.free_zero_bytes as f64) / (self.physical_total_bytes as f64)
        }
    }
}

/// Plain-language diagnostic system state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SystemState {
    /// System memory is ample and operating comfortably.
    #[default]
    Healthy,
    /// Large standby cache occupying memory; completely normal caching behavior.
    CacheFull,
    /// Available memory is critically low; background applications may need trimming.
    UnderPressure,
    /// Commit limit is dangerously near; system at risk of out-of-memory allocation failures.
    CommitRisk,
}

impl SystemState {
    /// Human-readable state name.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Healthy => "Healthy",
            Self::CacheFull => "Cache Full (Normal)",
            Self::UnderPressure => "Under Pressure",
            Self::CommitRisk => "Commit Limit Risk",
        }
    }
}

/// Comprehensive state diagnosis with explanation and actionable guidance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateDiagnosis {
    /// The classified system state.
    pub state: SystemState,
    /// Short plain-language headline.
    pub headline: String,
    /// Detailed diagnostic explanation.
    pub explanation: String,
    /// Top offending committers if state is CommitRisk.
    pub commit_offenders: Vec<ProcessConsumer>,
}
