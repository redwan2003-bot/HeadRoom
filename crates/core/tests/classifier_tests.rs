//! Unit tests for memory state classifier, hysteresis filter, and configuration.

use headroom_core::{
    classify, Config, HysteresisFilter, MemorySample, SystemState, CURRENT_SCHEMA_VERSION,
};

fn load_sample(fixture_json: &str) -> MemorySample {
    serde_json::from_str(fixture_json).expect("Failed to deserialize fixture sample")
}

#[test]
fn test_classify_healthy() {
    let raw = include_str!("fixtures/trace_healthy.json");
    let sample = load_sample(raw);
    let cfg = Config::default();

    let diag = classify(&sample, &cfg);
    assert_eq!(diag.state, SystemState::Healthy);
    assert!(diag.commit_offenders.is_empty());
}

#[test]
fn test_classify_cache_full() {
    let raw = include_str!("fixtures/trace_cache_full.json");
    let sample = load_sample(raw);
    let cfg = Config::default();

    let diag = classify(&sample, &cfg);
    assert_eq!(diag.state, SystemState::CacheFull);
    assert!(diag.explanation.contains("caching"));
}

#[test]
fn test_classify_under_pressure() {
    let raw = include_str!("fixtures/trace_under_pressure.json");
    let sample = load_sample(raw);
    let cfg = Config::default();

    let diag = classify(&sample, &cfg);
    assert_eq!(diag.state, SystemState::UnderPressure);
}

#[test]
fn test_classify_commit_risk() {
    let raw = include_str!("fixtures/trace_commit_risk.json");
    let sample = load_sample(raw);
    let cfg = Config::default();

    let diag = classify(&sample, &cfg);
    assert_eq!(diag.state, SystemState::CommitRisk);
    assert!(!diag.commit_offenders.is_empty());
    assert_eq!(diag.commit_offenders[0].name, "mem_hog.exe");
    assert!(diag
        .explanation
        .contains("Trimming working sets will not lower commit charge"));
}

#[test]
fn test_hysteresis_filter() {
    let healthy_sample = load_sample(include_str!("fixtures/trace_healthy.json"));
    let pressure_sample = load_sample(include_str!("fixtures/trace_under_pressure.json"));
    let commit_risk_sample = load_sample(include_str!("fixtures/trace_commit_risk.json"));
    let cfg = Config::default();

    let mut filter = HysteresisFilter::new(3);

    // Initial state is Healthy
    let diag_healthy = classify(&healthy_sample, &cfg);
    assert_eq!(filter.update(&diag_healthy), SystemState::Healthy);

    // One sample of pressure does not flip immediately (needs 3 consecutive)
    let diag_pressure = classify(&pressure_sample, &cfg);
    assert_eq!(filter.update(&diag_pressure), SystemState::Healthy);
    assert_eq!(filter.update(&diag_pressure), SystemState::Healthy);
    // 3rd consecutive sample confirms transition to UnderPressure
    assert_eq!(filter.update(&diag_pressure), SystemState::UnderPressure);

    // CommitRisk bypasses debounce due to high urgency
    let diag_commit = classify(&commit_risk_sample, &cfg);
    assert_eq!(filter.update(&diag_commit), SystemState::CommitRisk);
}

#[test]
fn test_config_fallback_and_migration() {
    // Corrupted JSON fallback
    let (cfg, warn) = Config::from_json_with_fallback("{ invalid json");
    assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
    assert!(warn.is_some());
    assert!(warn.unwrap().contains("fallback to defaults"));

    // Migration from old schema version
    let old_json = r#"{
        "schema_version": 0,
        "sample_interval_ms": 1500,
        "hysteresis_samples": 2,
        "thresholds": {
            "commit_risk_percent": 95,
            "under_pressure_percent": 8,
            "under_pressure_floor_mb": 256,
            "cache_full_standby_percent": 35,
            "cache_full_free_percent": 4
        },
        "auto_clean_enabled": false,
        "stutter_profile_enabled": false,
        "exclusions": ["test.exe"]
    }"#;
    let (migrated, warn_migrated) = Config::from_json_with_fallback(old_json);
    assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(migrated.sample_interval_ms, 1500);
    assert!(warn_migrated.is_some());
    assert!(warn_migrated
        .unwrap()
        .contains("migrated to current version"));
}
