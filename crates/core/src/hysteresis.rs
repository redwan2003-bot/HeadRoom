//! Hysteresis debouncing for state transitions.
//!
//! Prevents rapid UI and notification flickering around threshold boundaries.

use crate::types::{StateDiagnosis, SystemState};

/// Manages state transitions with a confirmation window.
#[derive(Debug, Clone)]
pub struct HysteresisFilter {
    /// Currently confirmed system state.
    current_state: SystemState,
    /// Pending candidate state awaiting debounce confirmation.
    pending_state: SystemState,
    /// Number of consecutive occurrences of pending_state.
    pending_count: u32,
    /// Threshold of consecutive occurrences required to confirm transition.
    required_samples: u32,
}

impl HysteresisFilter {
    /// Creates a new hysteresis filter with a designated confirmation sample requirement.
    pub fn new(required_samples: u32) -> Self {
        Self {
            current_state: SystemState::Healthy,
            pending_state: SystemState::Healthy,
            pending_count: 0,
            required_samples: required_samples.max(1),
        }
    }

    /// Feeds a new diagnosis and returns the debounced active state.
    pub fn update(&mut self, diagnosis: &StateDiagnosis) -> SystemState {
        let proposed = diagnosis.state;

        // Urgent states (CommitRisk) bypass debounce to alert promptly
        if proposed == SystemState::CommitRisk && self.current_state != SystemState::CommitRisk {
            self.current_state = proposed;
            self.pending_state = proposed;
            self.pending_count = 0;
            return self.current_state;
        }

        if proposed == self.current_state {
            self.pending_state = proposed;
            self.pending_count = 0;
            return self.current_state;
        }

        if proposed == self.pending_state {
            self.pending_count = self.pending_count.saturating_add(1);
            if self.pending_count >= self.required_samples {
                self.current_state = proposed;
                self.pending_count = 0;
            }
        } else {
            self.pending_state = proposed;
            self.pending_count = 1;
            if self.required_samples <= 1 {
                self.current_state = proposed;
            }
        }

        self.current_state
    }

    /// Returns the currently active confirmed state.
    pub fn active_state(&self) -> SystemState {
        self.current_state
    }
}
