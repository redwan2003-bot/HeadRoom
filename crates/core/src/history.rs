//! Pre-allocated circular ring buffer for 24-hour local telemetry history.
//!
//! # Footprint & Guarantees
//! - Pre-allocated fixed buffer: < 50 KB total memory.
//! - Zero heap allocations during runtime sampling.
//! - Strictly `#![forbid(unsafe_code)]`.
//! - Provides high-resolution (10-minute) and low-resolution (24-hour) views.

use crate::types::MemorySample;
use serde::{Deserialize, Serialize};

/// High-density history sample point (24 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HistoryPoint {
    pub timestamp_utc: u64,
    pub available_mb: u32,
    pub standby_mb: u32,
    pub commit_mb: u32,
    pub hard_faults_per_sec: f32,
}

impl Default for HistoryPoint {
    fn default() -> Self {
        Self {
            timestamp_utc: 0,
            available_mb: 0,
            standby_mb: 0,
            commit_mb: 0,
            hard_faults_per_sec: 0.0,
        }
    }
}

/// Accumulator for calculating 1-minute averages from 1-second ticks.
#[derive(Debug, Clone, Default)]
struct MinuteAccumulator {
    count: u32,
    avail_sum: u64,
    standby_sum: u64,
    commit_sum: u64,
    faults_sum: f64,
}

impl MinuteAccumulator {
    fn add(&mut self, pt: &HistoryPoint) {
        self.count += 1;
        self.avail_sum += pt.available_mb as u64;
        self.standby_sum += pt.standby_mb as u64;
        self.commit_sum += pt.commit_mb as u64;
        self.faults_sum += pt.hard_faults_per_sec as f64;
    }

    fn flush(&mut self, timestamp_utc: u64) -> Option<HistoryPoint> {
        if self.count == 0 {
            return None;
        }

        let pt = HistoryPoint {
            timestamp_utc,
            available_mb: (self.avail_sum / self.count as u64) as u32,
            standby_mb: (self.standby_sum / self.count as u64) as u32,
            commit_mb: (self.commit_sum / self.count as u64) as u32,
            hard_faults_per_sec: (self.faults_sum / self.count as f64) as f32,
        };

        *self = Self::default();
        Some(pt)
    }
}

/// Circular history storage for telemetry trends.
pub struct MemoryHistory {
    /// 1-second resolution for last 10 minutes (600 points).
    high_res: [HistoryPoint; 600],
    high_res_head: usize,
    high_res_count: usize,

    /// 1-minute resolution for last 24 hours (1440 points).
    low_res: [HistoryPoint; 1440],
    low_res_head: usize,
    low_res_count: usize,

    acc: MinuteAccumulator,
}

impl Default for MemoryHistory {
    fn default() -> Self {
        Self {
            high_res: [HistoryPoint::default(); 600],
            high_res_head: 0,
            high_res_count: 0,
            low_res: [HistoryPoint::default(); 1440],
            low_res_head: 0,
            low_res_count: 0,
            acc: MinuteAccumulator::default(),
        }
    }
}

impl MemoryHistory {
    /// Records an observation tick from the sampler.
    pub fn record(&mut self, sample: &MemorySample) {
        let pt = HistoryPoint {
            timestamp_utc: sample.timestamp_utc,
            available_mb: (sample.available_bytes / (1024 * 1024)) as u32,
            standby_mb: (sample.standby.total() / (1024 * 1024)) as u32,
            commit_mb: (sample.commit.total_bytes / (1024 * 1024)) as u32,
            hard_faults_per_sec: sample.hard_faults_per_sec as f32,
        };

        // 1. High-resolution ring (600 seconds = 10 minutes)
        self.high_res[self.high_res_head] = pt;
        self.high_res_head = (self.high_res_head + 1) % 600;
        if self.high_res_count < 600 {
            self.high_res_count += 1;
        }

        // 2. Accumulate into 1-minute bucket (60 seconds)
        self.acc.add(&pt);
        if self.acc.count >= 60 {
            if let Some(avg_pt) = self.acc.flush(sample.timestamp_utc) {
                self.low_res[self.low_res_head] = avg_pt;
                self.low_res_head = (self.low_res_head + 1) % 1440;
                if self.low_res_count < 1440 {
                    self.low_res_count += 1;
                }
            }
        }
    }

    /// Retrieves chronological points for the last 10 minutes.
    pub fn get_recent_10_minutes(&self) -> Vec<HistoryPoint> {
        let mut result = Vec::with_capacity(self.high_res_count);
        if self.high_res_count == 0 {
            return result;
        }

        let start_idx = if self.high_res_count < 600 {
            0
        } else {
            self.high_res_head
        };

        for i in 0..self.high_res_count {
            let idx = (start_idx + i) % 600;
            result.push(self.high_res[idx]);
        }

        result
    }

    /// Retrieves chronological points for the last 24 hours.
    pub fn get_history_24_hours(&self) -> Vec<HistoryPoint> {
        let mut result = Vec::with_capacity(self.low_res_count);
        if self.low_res_count == 0 {
            return result;
        }

        let start_idx = if self.low_res_count < 1440 {
            0
        } else {
            self.low_res_head
        };

        for i in 0..self.low_res_count {
            let idx = (start_idx + i) % 1440;
            result.push(self.low_res[idx]);
        }

        result
    }

    /// Returns the number of recorded 1-second and 1-minute samples.
    pub fn counts(&self) -> (usize, usize) {
        (self.high_res_count, self.low_res_count)
    }
}
