//! Comprehensive unit tests for policy decisions, modes, and history buffers.

use headroom_core::config::{Config, OperatingMode};
use headroom_core::history::MemoryHistory;
use headroom_core::policy::{decide, PolicyDecision, PolicyState};
use headroom_core::types::{CommitInfo, MemorySample, ProcessConsumer, StandbyBreakdown};

fn create_sample(
    avail_mb: u64,
    free_mb: u64,
    standby_mb: u64,
    commit_used_mb: u64,
    commit_limit_mb: u64,
) -> MemorySample {
    MemorySample {
        timestamp_utc: 1000,
        physical_total_bytes: 16 * 1024 * 1024 * 1024, // 16 GB
        available_bytes: avail_mb * 1024 * 1024,
        in_use_bytes: (16 * 1024 - avail_mb) * 1024 * 1024,
        modified_bytes: 100 * 1024 * 1024,
        standby: StandbyBreakdown {
            reserve_bytes: 0,
            normal_bytes: standby_mb * 1024 * 1024,
            core_bytes: 0,
        },
        free_zero_bytes: free_mb * 1024 * 1024,
        commit: CommitInfo {
            total_bytes: commit_used_mb * 1024 * 1024,
            limit_bytes: commit_limit_mb * 1024 * 1024,
            peak_bytes: commit_used_mb * 1024 * 1024,
        },
        hard_faults_per_sec: 0.0,
        top_consumers: vec![
            ProcessConsumer {
                pid: 100,
                name: "active_editor.exe".to_string(),
                private_bytes: 800 * 1024 * 1024,
                working_set_bytes: 600 * 1024 * 1024,
                is_growing: false,
            },
            ProcessConsumer {
                pid: 200,
                name: "background_browser.exe".to_string(),
                private_bytes: 600 * 1024 * 1024,
                working_set_bytes: 500 * 1024 * 1024,
                is_growing: false,
            },
            ProcessConsumer {
                pid: 300,
                name: "explorer.exe".to_string(),
                private_bytes: 200 * 1024 * 1024,
                working_set_bytes: 150 * 1024 * 1024,
                is_growing: false,
            },
        ],
    }
}

#[test]
fn test_policy_commit_risk_never_cleans() {
    let cfg = Config {
        mode: OperatingMode::Automatic,
        ..Default::default()
    };
    let state = PolicyState::default();

    // Commit charge 95% (20.9 GB / 22 GB)
    let sample = create_sample(400, 100, 300, 20900, 22000);

    let decision = decide(&sample, &cfg, &state, false, Some(100), false);

    match decision {
        PolicyDecision::WarnCommitRisk { top_committers, .. } => {
            assert!(!top_committers.is_empty());
        }
        other => panic!("Expected WarnCommitRisk, got {:?}", other),
    }
}

#[test]
fn test_policy_monitor_mode_does_nothing() {
    let cfg = Config {
        mode: OperatingMode::Monitor,
        ..Default::default()
    };
    let state = PolicyState::default();

    // Memory critically low (300 MB available)
    let sample = create_sample(300, 50, 250, 5000, 20000);

    let decision = decide(&sample, &cfg, &state, false, Some(100), false);
    assert_eq!(decision, PolicyDecision::None);
}

#[test]
fn test_policy_fullscreen_gaming_suppression() {
    let cfg = Config {
        mode: OperatingMode::Automatic,
        ..Default::default()
    };
    let state = PolicyState::default();

    // Low available memory while running a fullscreen game
    let sample = create_sample(300, 50, 250, 5000, 20000);
    let is_fullscreen = true;

    let decision = decide(&sample, &cfg, &state, is_fullscreen, Some(100), false);
    assert_eq!(decision, PolicyDecision::None);
}

#[test]
fn test_policy_foreground_app_protected_and_exclusions_honored() {
    let cfg = Config {
        mode: OperatingMode::Automatic,
        ..Default::default()
    };
    let state = PolicyState::default();

    // Low available memory, foreground is PID 100 ("active_editor.exe")
    // "explorer.exe" is in default exclusions
    let sample = create_sample(400, 50, 350, 5000, 20000);

    let decision = decide(&sample, &cfg, &state, false, Some(100), false);

    match decision {
        PolicyDecision::TrimBackgroundApps {
            target_pids,
            target_names,
            ..
        } => {
            // Must contain background_browser (PID 200)
            assert!(target_pids.contains(&200));
            assert!(target_names.contains(&"background_browser.exe".to_string()));

            // Must NOT contain foreground active_editor (PID 100)
            assert!(!target_pids.contains(&100));

            // Must NOT contain excluded explorer.exe (PID 300)
            assert!(!target_pids.contains(&300));
        }
        other => panic!("Expected TrimBackgroundApps, got {:?}", other),
    }
}

#[test]
fn test_policy_stutter_profile_purges_low_standby() {
    let cfg = Config {
        mode: OperatingMode::Automatic,
        stutter_profile_enabled: true,
        ..Default::default()
    };
    let state = PolicyState::default();

    // Available memory is OK (3500 MB), but Free is almost gone (100 MB)
    // while Standby cache is 3400 MB
    let sample = create_sample(3500, 100, 3400, 5000, 20000);

    let decision = decide(&sample, &cfg, &state, false, Some(100), false);

    match decision {
        PolicyDecision::PurgeLowPriorityStandby { .. } => {}
        other => panic!("Expected PurgeLowPriorityStandby, got {:?}", other),
    }
}

#[test]
fn test_policy_kernel_low_memory_triggers_trim() {
    let cfg = Config {
        mode: OperatingMode::Automatic,
        ..Default::default()
    };
    let state = PolicyState::default();

    // Available memory is above user threshold (e.g. 2000 MB), but kernel signals low memory
    let sample = create_sample(2000, 1000, 1000, 5000, 20000);

    let decision = decide(&sample, &cfg, &state, false, Some(100), true);

    match decision {
        PolicyDecision::TrimBackgroundApps { target_pids, .. } => {
            assert!(target_pids.contains(&200));
        }
        other => panic!(
            "Expected TrimBackgroundApps triggered by kernel signal, got {:?}",
            other
        ),
    }
}

#[test]
fn test_history_ring_buffer_rollover() {
    let mut history = MemoryHistory::default();

    // Record 700 ticks (> 600 capacity of high-res ring)
    for i in 0..700 {
        let sample = create_sample(4000 + (i % 500), 500, 3500, 6000, 20000);
        history.record(&sample);
    }

    let (high_count, low_count) = history.counts();
    assert_eq!(high_count, 600); // capped at capacity
    assert!(low_count >= 11); // 700 / 60 = 11 full minutes

    let recent = history.get_recent_10_minutes();
    assert_eq!(recent.len(), 600);

    let low_res = history.get_history_24_hours();
    assert_eq!(low_res.len(), low_count);
}

#[test]
fn test_config_exclusion_management() {
    let mut cfg = Config::default();

    // Default exclusions include "explorer.exe"
    assert!(cfg.is_excluded("explorer.exe"));
    assert!(cfg.is_excluded("EXPLORER.EXE")); // Case-insensitive

    // Add new exclusion
    assert!(cfg.add_exclusion("game.exe"));
    assert!(!cfg.add_exclusion("GAME.EXE")); // Duplicate prevented
    assert!(cfg.is_excluded("game.exe"));

    // Toggle exclusion
    let now_excluded = cfg.toggle_exclusion("game.exe");
    assert!(!now_excluded);
    assert!(!cfg.is_excluded("game.exe"));

    let now_excluded_again = cfg.toggle_exclusion("game.exe");
    assert!(now_excluded_again);
    assert!(cfg.is_excluded("game.exe"));

    // Remove exclusion
    assert!(cfg.remove_exclusion("game.exe"));
    assert!(!cfg.remove_exclusion("game.exe")); // Already removed
}

#[test]
fn test_config_load_or_default() {
    let (cfg, warning) = Config::load_or_default();
    assert_eq!(warning, None);
    assert_eq!(cfg.schema_version, 1);
    assert!(cfg.is_excluded("explorer.exe"));
}
