//! Headroom Sys: Safe Win32 telemetry and process query wrappers.
//!
//! # Unsafe Policy
//! All Win32 and NT unsafe calls are quarantined in this crate. Every `unsafe` block
//! is audited and annotated with a required `// SAFETY:` soundness justification.

pub mod counters;
pub mod error;
pub mod notification;
pub mod perf_info;
pub mod processes;

pub use counters::{MemoryCountersSnapshot, PdhQueryManager};
pub use error::SysError;
pub use notification::is_fullscreen_or_gaming_active;
pub use perf_info::{query_performance_info, SystemPerformanceSnapshot};
pub use processes::query_process_consumers;

use headroom_core::types::{MemorySample, StandbyBreakdown};
use std::time::{SystemTime, UNIX_EPOCH};

/// High-level memory sampler orchestrating PDH counters and PSAPI metrics.
pub struct SystemSampler {
    pdh: Option<PdhQueryManager>,
}

impl SystemSampler {
    /// Initializes full sampler with persistent PDH queries for dashboard display.
    pub fn new() -> Result<Self, SysError> {
        let pdh = PdhQueryManager::new().ok();
        Ok(Self { pdh })
    }

    /// Initializes ultra-lightweight sampler using GetPerformanceInfo for the background agent,
    /// avoiding PDH provider DLL mapping to keep private bytes comfortably under 5 MB.
    pub fn new_lightweight() -> Result<Self, SysError> {
        Ok(Self { pdh: None })
    }

    /// Samples current memory metrics, constructing a complete `MemorySample`.
    pub fn sample(&mut self, top_process_limit: usize) -> Result<MemorySample, SysError> {
        let perf = query_performance_info()?;
        let top_consumers = if top_process_limit > 0 {
            query_process_consumers(top_process_limit).unwrap_or_default()
        } else {
            Vec::new()
        };

        let timestamp_utc = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let available_bytes = perf.physical_available_bytes;
        let in_use_bytes = perf.physical_total_bytes.saturating_sub(available_bytes);

        if let Some(pdh) = &mut self.pdh {
            let counters = pdh.collect()?;
            let standby_total = available_bytes.saturating_sub(counters.free_zero_bytes);

            Ok(MemorySample {
                timestamp_utc,
                physical_total_bytes: perf.physical_total_bytes,
                available_bytes,
                in_use_bytes,
                modified_bytes: counters.modified_bytes,
                standby: StandbyBreakdown {
                    reserve_bytes: 0,
                    normal_bytes: standby_total,
                    core_bytes: 0,
                },
                free_zero_bytes: counters.free_zero_bytes,
                commit: perf.commit,
                hard_faults_per_sec: counters.pages_input_per_sec,
                top_consumers,
            })
        } else {
            // Lightweight PSAPI estimation
            let standby_est = (perf.physical_available_bytes / 2).max(1);
            let free_est = perf.physical_available_bytes.saturating_sub(standby_est);

            Ok(MemorySample {
                timestamp_utc,
                physical_total_bytes: perf.physical_total_bytes,
                available_bytes,
                in_use_bytes,
                modified_bytes: 0,
                standby: StandbyBreakdown {
                    reserve_bytes: 0,
                    normal_bytes: standby_est,
                    core_bytes: 0,
                },
                free_zero_bytes: free_est,
                commit: perf.commit,
                hard_faults_per_sec: 0.0,
                top_consumers,
            })
        }
    }
}
