//! Versioned configuration and thresholds.
//!
//! Provides safe defaults, schema versioning, and fallback recovery.

use serde::{Deserialize, Serialize};

/// Current configuration schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Configurable memory pressure and cleaning thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thresholds {
    /// Commit percentage threshold (0-100) above which CommitRisk state is entered.
    pub commit_risk_percent: u8,
    /// Available memory percentage (0-100) below which UnderPressure is entered.
    pub under_pressure_percent: u8,
    /// Minimum available MB floor below which UnderPressure is triggered regardless of percentage.
    pub under_pressure_floor_mb: u64,
    /// Standby cache percentage (0-100) to qualify for CacheFull diagnostic.
    pub cache_full_standby_percent: u8,
    /// Maximum Free/Zero percentage (0-100) to qualify for CacheFull diagnostic.
    pub cache_full_free_percent: u8,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            commit_risk_percent: 90,
            under_pressure_percent: 10,
            under_pressure_floor_mb: 512,
            cache_full_standby_percent: 30,
            cache_full_free_percent: 5,
        }
    }
}

/// Operating mode for Headroom memory management.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OperatingMode {
    /// Telemetry and diagnostics only; never acts automatically (Default).
    #[default]
    Monitor,
    /// Shows notifications / prompts when actions are recommended.
    AskFirst,
    /// Executes safe cleaning actions automatically.
    Automatic,
}

/// Persistent user configuration for Headroom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Schema version for migrations.
    pub schema_version: u32,
    /// Operating mode (Monitor, AskFirst, Automatic).
    #[serde(default)]
    pub mode: OperatingMode,
    /// Telemetry polling interval in milliseconds.
    pub sample_interval_ms: u32,
    /// Consecutive samples required to trigger state transition (hysteresis).
    pub hysteresis_samples: u32,
    /// Diagnostic and policy thresholds.
    pub thresholds: Thresholds,
    /// Automatic cleaning enabled (legacy flag; synced with mode == Automatic).
    pub auto_clean_enabled: bool,
    /// Opt-in gamer stutter reduction profile (default: false).
    pub stutter_profile_enabled: bool,
    /// Excluded process names (case-insensitive, e.g. "game.exe").
    pub exclusions: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            mode: OperatingMode::Monitor,
            sample_interval_ms: 1000,
            hysteresis_samples: 3,
            thresholds: Thresholds::default(),
            auto_clean_enabled: false, // Locked decision: Default mode is Monitor only
            stutter_profile_enabled: false,
            exclusions: vec![
                "explorer.exe".to_string(),
                "dwm.exe".to_string(),
                "headroom-agent.exe".to_string(),
            ],
        }
    }
}

impl Config {
    /// Deserializes a configuration from JSON, migrating older versions or
    /// falling back safely to defaults if corrupted.
    pub fn from_json_with_fallback(json_str: &str) -> (Self, Option<String>) {
        if json_str.trim().is_empty() {
            return (Self::default(), None);
        }

        match serde_json::from_str::<Config>(json_str) {
            Ok(mut cfg) => {
                if cfg.schema_version != CURRENT_SCHEMA_VERSION {
                    let warning = format!(
                        "Config schema version {} migrated to current version {}",
                        cfg.schema_version, CURRENT_SCHEMA_VERSION
                    );
                    cfg.schema_version = CURRENT_SCHEMA_VERSION;
                    (cfg, Some(warning))
                } else {
                    (cfg, None)
                }
            }
            Err(e) => {
                let warning = format!(
                    "Failed to parse config (fallback to defaults applied): {}",
                    e
                );
                (Self::default(), Some(warning))
            }
        }
    }

    /// Serializes configuration to pretty-printed JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
