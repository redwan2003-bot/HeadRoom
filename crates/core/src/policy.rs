//! Pure functional policy engine for Headroom.
//!
//! # Evaluation Rules & Precedence
//! 1. Commit Near Limit: Warn and name top committers. NEVER clean.
//! 2. Available Memory Low: Trim background apps, skipping exclusions and foreground window.
//! 3. Free Low, Standby High & Stutter Profile Active: Purge low-priority standby.
//! 4. Otherwise: Do nothing.
//!
//! # Safety Invariants
//! - Automatic mode never touches the active foreground app.
//! - Automatic mode unconditionally pauses during fullscreen games or presentations.
//! - Monitor mode never initiates any cleaning actions.

use crate::config::{Config, OperatingMode};
use crate::types::MemorySample;

/// Decision emitted by the policy engine for an observation tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    /// No cleaning action required or action suppressed by safety guards.
    None,

    /// System commit charge is nearing exhaustion.
    /// Invariant: Never clean under commit risk; warn and name consumers instead.
    WarnCommitRisk {
        top_committers: Vec<String>,
        reason: String,
    },

    /// Background applications should be trimmed to relieve physical memory pressure.
    TrimBackgroundApps {
        target_pids: Vec<u32>,
        target_names: Vec<String>,
        reason: String,
    },

    /// Low-priority standby pages should be purged to eliminate stutter for games.
    PurgeLowPriorityStandby { reason: String },
}

/// Dynamic state tracked across policy evaluations.
#[derive(Debug, Clone)]
pub struct PolicyState {
    /// Timestamp of the last executed clean action.
    pub last_action_timestamp: u64,
    /// Base cooldown between automatic actions in seconds.
    pub base_cooldown_secs: u64,
    /// Adaptive multiplier applied when post-clean readback is elevated.
    pub cooldown_multiplier: f64,
    /// Number of consecutive ticks under pressure (hysteresis).
    pub consecutive_under_pressure: u32,
    /// Number of consecutive ticks at commit risk.
    pub consecutive_commit_risk: u32,
}

impl Default for PolicyState {
    fn default() -> Self {
        Self {
            last_action_timestamp: 0,
            base_cooldown_secs: 60,
            cooldown_multiplier: 1.0,
            consecutive_under_pressure: 0,
            consecutive_commit_risk: 0,
        }
    }
}

impl PolicyState {
    /// Returns the effective cooldown period in seconds.
    pub fn effective_cooldown_secs(&self) -> u64 {
        ((self.base_cooldown_secs as f64) * self.cooldown_multiplier).round() as u64
    }

    /// Checks if the engine is currently in a cooldown window.
    pub fn is_in_cooldown(&self, current_timestamp: u64) -> bool {
        current_timestamp.saturating_sub(self.last_action_timestamp)
            < self.effective_cooldown_secs()
    }
}

/// Evaluates system metrics against configuration and state to determine the appropriate action.
///
/// This is a pure function: no OS calls, no side effects, deterministic execution.
pub fn decide(
    sample: &MemorySample,
    cfg: &Config,
    state: &PolicyState,
    is_fullscreen_or_gaming: bool,
    foreground_pid: Option<u32>,
    is_kernel_low_memory: bool,
) -> PolicyDecision {
    let commit_pct = sample.commit.percent();
    let avail_bytes = sample.available_bytes;
    let total_bytes = sample.physical_total_bytes;
    let avail_pct = if total_bytes > 0 {
        ((avail_bytes as f64 / total_bytes as f64) * 100.0).round() as u8
    } else {
        100
    };

    // 1. RULE 1: Commit near limit -> WARN, name top committers, NEVER clean.
    if commit_pct >= cfg.thresholds.commit_risk_percent {
        let top_committers: Vec<String> = sample
            .top_consumers
            .iter()
            .take(3)
            .map(|p| format!("{} ({} MB)", p.name, p.private_bytes / (1024 * 1024)))
            .collect();

        return PolicyDecision::WarnCommitRisk {
            top_committers,
            reason: format!(
                "Commit charge is critically high ({}% of commit limit). Trimming does not reduce commit charge.",
                commit_pct
            ),
        };
    }

    // Monitor mode never initiates cleaning
    if cfg.mode == OperatingMode::Monitor {
        return PolicyDecision::None;
    }

    // Safety Invariant: Automatic mode pauses during fullscreen gaming/presentation
    if is_fullscreen_or_gaming {
        return PolicyDecision::None;
    }

    // Cooldown check
    if state.is_in_cooldown(sample.timestamp_utc) {
        return PolicyDecision::None;
    }

    // 2. RULE 2: Available memory low -> Trim background apps, skipping exclusions and foreground.
    let under_pressure_floor_bytes = cfg.thresholds.under_pressure_floor_mb * 1024 * 1024;
    let is_under_pressure = avail_pct <= cfg.thresholds.under_pressure_percent
        || avail_bytes <= under_pressure_floor_bytes
        || is_kernel_low_memory;

    if is_under_pressure {
        // Collect eligible background processes (exclude foreground & user exclusions)
        let mut target_pids = Vec::new();
        let mut target_names = Vec::new();

        for proc in &sample.top_consumers {
            // Never touch the active foreground window
            if Some(proc.pid) == foreground_pid {
                continue;
            }

            // Check exclusions (case-insensitive)
            let is_excluded = cfg
                .exclusions
                .iter()
                .any(|ex| ex.eq_ignore_ascii_case(&proc.name));
            if is_excluded {
                continue;
            }

            // Only trim processes holding meaningful working set (> 50 MB)
            if proc.working_set_bytes > 50 * 1024 * 1024 {
                target_pids.push(proc.pid);
                target_names.push(proc.name.clone());
            }
        }

        if !target_pids.is_empty() {
            return PolicyDecision::TrimBackgroundApps {
                target_pids,
                target_names,
                reason: format!(
                    "Physical available memory is low ({} MB, {}%). Relieving background working sets.",
                    avail_bytes / (1024 * 1024),
                    avail_pct
                ),
            };
        }
    }

    // 3. RULE 3: Free low, standby large & stutter profile active -> Purge low-priority standby.
    if cfg.stutter_profile_enabled {
        let free_bytes = sample.free_zero_bytes;
        let standby_bytes = sample.standby.total();

        // Trigger condition: Free < 250 MB and Standby > 2 GB (cache is occupying space that free-list needs)
        let free_is_low = free_bytes < 250 * 1024 * 1024;
        let standby_is_large = standby_bytes > 2048 * 1024 * 1024;

        if free_is_low && standby_is_large {
            return PolicyDecision::PurgeLowPriorityStandby {
                reason: format!(
                    "Stutter profile active: Free RAM is low ({} MB) while standby cache is large ({} MB).",
                    free_bytes / (1024 * 1024),
                    standby_bytes / (1024 * 1024)
                ),
            };
        }
    }

    // 4. RULE 4: Otherwise -> Do nothing.
    PolicyDecision::None
}
