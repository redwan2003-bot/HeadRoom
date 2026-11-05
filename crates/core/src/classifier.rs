//! Memory state diagnosis and classification engine.
//!
//! Pure functional evaluation mapping system metrics to human-readable states.

use crate::config::Config;
use crate::types::{MemorySample, StateDiagnosis, SystemState};

/// Diagnoses a single memory sample against active configuration thresholds.
///
/// Order of evaluation strictly follows Section 8 of the Headroom specification:
/// 1. Commit near its limit: **CommitRisk** warning, names top committers, does not clean.
/// 2. Available memory low: **UnderPressure**.
/// 3. Free low and Standby large: **CacheFull** (healthy caching).
/// 4. Otherwise: **Healthy**.
pub fn classify(sample: &MemorySample, cfg: &Config) -> StateDiagnosis {
    let thresholds = &cfg.thresholds;

    // 1. Commit charge check
    let commit_pct = sample.commit.percent();
    if commit_pct >= thresholds.commit_risk_percent {
        let mut offenders = sample.top_consumers.clone();
        offenders.sort_by_key(|a| std::cmp::Reverse(a.private_bytes));
        offenders.truncate(5);

        let offender_names: Vec<String> = offenders
            .iter()
            .map(|p| format!("{} ({} MB)", p.name, p.private_bytes / (1024 * 1024)))
            .collect();

        return StateDiagnosis {
            state: SystemState::CommitRisk,
            headline: format!("Commit Limit Warning ({}% Committed)", commit_pct),
            explanation: format!(
                "System committed memory is near the paging limit. Trimming working sets will not lower commit charge. Top committers: {}.",
                offender_names.join(", ")
            ),
            commit_offenders: offenders,
        };
    }

    // 2. Physical memory pressure check
    let avail_pct = (sample.available_ratio() * 100.0) as u8;
    let avail_mb = sample.available_bytes / (1024 * 1024);
    if avail_pct <= thresholds.under_pressure_percent
        || avail_mb <= thresholds.under_pressure_floor_mb
    {
        return StateDiagnosis {
            state: SystemState::UnderPressure,
            headline: format!("Memory Under Pressure ({} MB Available)", avail_mb),
            explanation: format!(
                "Only {} MB ({:.1}%) of physical memory is currently available. Active processes may experience page faults if memory is not freed.",
                avail_mb,
                sample.available_ratio() * 100.0
            ),
            commit_offenders: Vec::new(),
        };
    }

    // 3. Cache full diagnostic check
    let standby_pct = (sample.standby_ratio() * 100.0) as u8;
    let free_pct = (sample.free_zero_ratio() * 100.0) as u8;
    if standby_pct >= thresholds.cache_full_standby_percent
        && free_pct <= thresholds.cache_full_free_percent
    {
        let standby_mb = sample.standby.total() / (1024 * 1024);
        return StateDiagnosis {
            state: SystemState::CacheFull,
            headline: format!("Standby Cache Full ({} MB Cached)", standby_mb),
            explanation: format!(
                "Windows is actively caching previously accessed files in Standby memory ({} MB). This memory remains fully available and will be automatically released when applications need it.",
                standby_mb
            ),
            commit_offenders: Vec::new(),
        };
    }

    // 4. Default: Healthy
    StateDiagnosis {
        state: SystemState::Healthy,
        headline: "System Memory Healthy".to_string(),
        explanation: format!(
            "Memory operating normally. {} MB available ({:.1}%), commit charge at {}%.",
            avail_mb,
            sample.available_ratio() * 100.0,
            commit_pct
        ),
        commit_offenders: Vec::new(),
    }
}
