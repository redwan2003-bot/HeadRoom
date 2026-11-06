//! Verifiable clean audit receipts logger.
//!
//! # Receipt Design
//! Writes JSON Lines to `%LOCALAPPDATA%\Headroom\receipts.jsonl` (or `./receipts.jsonl` in portable mode).
//! Every receipt includes memory freed, commit deltas, baseline page-in rates,
//! and subsequent read-back estimates with documented confidence notes.

use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// Current receipt schema version.
pub const RECEIPT_SCHEMA_VERSION: u32 = 1;

/// Audit receipt recording details of an executed memory action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CleanReceipt {
    pub schema_version: u32,
    pub timestamp_utc: u64,
    pub action: String,
    pub trigger: String,
    pub status: String,
    pub duration_ms: u64,
    pub available_before_mb: u64,
    pub available_after_mb: u64,
    pub commit_before_mb: u64,
    pub commit_after_mb: u64,
    pub baseline_pagein_rate: f64,
    pub readback_60s_estimate: f64,
    pub confidence_note: String,
}

impl CleanReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        action: &str,
        trigger: &str,
        status: &str,
        duration_ms: u64,
        avail_before: u64,
        avail_after: u64,
        commit_before: u64,
        commit_after: u64,
        baseline_pagein: f64,
        readback_estimate: f64,
    ) -> Self {
        Self {
            schema_version: RECEIPT_SCHEMA_VERSION,
            timestamp_utc: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            action: action.to_string(),
            trigger: trigger.to_string(),
            status: status.to_string(),
            duration_ms,
            available_before_mb: avail_before / (1024 * 1024),
            available_after_mb: avail_after / (1024 * 1024),
            commit_before_mb: commit_before / (1024 * 1024),
            commit_after_mb: commit_after / (1024 * 1024),
            baseline_pagein_rate: baseline_pagein,
            readback_60s_estimate: readback_estimate,
            confidence_note:
                "Read-back estimate is system-wide across all processes over 60 seconds."
                    .to_string(),
        }
    }
}

/// Appends an audit receipt to the receipts log with size-based rotation.
pub fn append_receipt(receipt: &CleanReceipt) -> std::io::Result<()> {
    let path = get_receipts_path();

    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }

    // Rotate if larger than 5 MB
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > 5 * 1024 * 1024 {
            let mut rotated = path.clone();
            rotated.set_extension("jsonl.1");
            let _ = std::fs::rename(&path, rotated);
        }
    }

    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;

    let json_line = serde_json::to_string(receipt)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    writeln!(file, "{}", json_line)?;
    Ok(())
}

fn get_receipts_path() -> PathBuf {
    if headroom_core::is_portable_mode() {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                return parent.join("receipts.jsonl");
            }
        }
    }

    if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local_appdata)
            .join("Headroom")
            .join("receipts.jsonl")
    } else {
        PathBuf::from("receipts.jsonl")
    }
}
