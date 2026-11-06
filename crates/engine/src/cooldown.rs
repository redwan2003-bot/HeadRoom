//! Action cooldown manager and rate limiter.
//!
//! Enforces cooldown intervals between cleaning actions and dynamically adjusts cooldowns
//! when high disk read-back is observed after a clean.

use headroom_ipc::ActionKind;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Manages cooldown timers across cleaning actions.
#[derive(Debug)]
pub struct CooldownManager {
    last_executed: HashMap<ActionKind, Instant>,
    base_cooldown: Duration,
    dynamic_penalty: Duration,
}

impl Default for CooldownManager {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}

impl CooldownManager {
    pub fn new(base_cooldown: Duration) -> Self {
        Self {
            last_executed: HashMap::new(),
            base_cooldown,
            dynamic_penalty: Duration::from_secs(0),
        }
    }

    /// Checks whether an action is currently allowed to execute.
    pub fn is_allowed(&self, action: ActionKind) -> bool {
        if let Some(last_time) = self.last_executed.get(&action) {
            let required = self.base_cooldown + self.dynamic_penalty;
            last_time.elapsed() >= required
        } else {
            true
        }
    }

    /// Records that an action was executed now.
    pub fn record_execution(&mut self, action: ActionKind) {
        self.last_executed.insert(action, Instant::now());
    }

    /// Adjusts dynamic penalty based on post-clean 60-second read-back surge.
    pub fn report_readback_surge(&mut self, pages_per_sec: f64) {
        if pages_per_sec > 50.0 {
            // High thrashing detected, add 30s penalty
            self.dynamic_penalty = Duration::from_secs(30);
        } else {
            self.dynamic_penalty = Duration::from_secs(0);
        }
    }
}
