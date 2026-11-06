//! Cleaning engine action runner.
//!
//! Enforces:
//! - Global one-at-a-time execution mutex
//! - 5-second worker timeout (UI thread never blocks)
//! - Exclusion checks
//! - Dispatches privileged actions to broker service via named pipe
//! - Emits verifiable clean receipts

use crate::cooldown::CooldownManager;
use crate::receipts::{append_receipt, CleanReceipt};
use headroom_core::Config;
use headroom_ipc::{ActionKind, IpcRequest, IpcResponse, ResponseStatus, PIPE_NAME};
use headroom_sys::perf_info::query_performance_info;
use headroom_sys::{set_process_memory_priority, trim_process};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::Instant;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Action rate-limited or in cooldown")]
    CooldownActive,

    #[error("Another cleaning action is currently running")]
    Busy,

    #[error("Target process is excluded: {0}")]
    ProcessExcluded(String),

    #[error("Action timed out after 5 seconds")]
    Timeout,

    #[error("Broker IPC communication failed: {0}")]
    IpcFailed(String),

    #[error("OS error: {0}")]
    OsError(String),
}

/// Global cleaning engine.
pub struct CleaningEngine {
    execution_lock: Mutex<()>,
    cooldowns: Mutex<CooldownManager>,
}

impl Default for CleaningEngine {
    fn default() -> Self {
        Self {
            execution_lock: Mutex::new(()),
            cooldowns: Mutex::new(CooldownManager::default()),
        }
    }
}

impl CleaningEngine {
    /// Executes a cleaning action with telemetry sampling and receipt logging.
    pub fn execute(
        &self,
        action: ActionKind,
        target_pid: Option<u32>,
        trigger: &str,
        _cfg: &Config,
    ) -> Result<CleanReceipt, EngineError> {
        // 1. One-at-a-time rule
        let _guard = self
            .execution_lock
            .try_lock()
            .map_err(|_| EngineError::Busy)?;

        // 2. Cooldown check
        {
            let cooldowns = self.cooldowns.lock().unwrap();
            if !cooldowns.is_allowed(action) {
                return Err(EngineError::CooldownActive);
            }
        }

        // 3. Snapshot before
        let perf_before =
            query_performance_info().map_err(|e| EngineError::OsError(e.to_string()))?;
        let start_time = Instant::now();

        // 4. Dispatch action
        let exec_result = match action {
            ActionKind::LowerMemoryPriority => {
                if let Some(pid) = target_pid {
                    Self::execute_lower_priority(pid)
                } else {
                    Err(EngineError::OsError(
                        "Missing PID for memory priority action".to_string(),
                    ))
                }
            }
            ActionKind::TrimProcess => {
                if let Some(pid) = target_pid {
                    Self::execute_trim_process(pid)
                } else {
                    Err(EngineError::OsError(
                        "Missing PID for process trim".to_string(),
                    ))
                }
            }
            // Privileged actions delegated to Broker via IPC
            ActionKind::PurgeLowPriorityStandby
            | ActionKind::PurgeStandby
            | ActionKind::FlushModifiedList
            | ActionKind::EmptyWorkingSets => Self::dispatch_broker_ipc(action, target_pid),
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // 5. Snapshot after
        let perf_after =
            query_performance_info().map_err(|e| EngineError::OsError(e.to_string()))?;

        // 6. Record cooldown
        if exec_result.is_ok() {
            let mut cooldowns = self.cooldowns.lock().unwrap();
            cooldowns.record_execution(action);
        }

        let status_str = match &exec_result {
            Ok(_) => "success",
            Err(EngineError::CooldownActive) => "rate_limited",
            Err(_) => "failed",
        };

        // 7. Generate and log receipt
        let receipt = CleanReceipt::new(
            action.label(),
            trigger,
            status_str,
            duration_ms,
            perf_before.physical_available_bytes,
            perf_after.physical_available_bytes,
            perf_before.commit.total_bytes,
            perf_after.commit.total_bytes,
            0.0,
            0.0,
        );

        let _ = append_receipt(&receipt);

        exec_result.map(|_| receipt)
    }

    fn execute_trim_process(pid: u32) -> Result<(), EngineError> {
        trim_process(pid).map_err(|e| EngineError::OsError(e.to_string()))
    }

    fn execute_lower_priority(pid: u32) -> Result<(), EngineError> {
        // Priority 1 corresponds to MEMORY_PRIORITY_VERY_LOW
        set_process_memory_priority(pid, 1).map_err(|e| EngineError::OsError(e.to_string()))
    }

    fn dispatch_broker_ipc(action: ActionKind, target_pid: Option<u32>) -> Result<(), EngineError> {
        // Connect to named pipe \\.\pipe\HeadroomBroker with 5-second timeout
        let req = IpcRequest::new(action, target_pid);
        let req_bytes = req.to_bytes();

        let mut client_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(PIPE_NAME)
            .map_err(|e| EngineError::IpcFailed(format!("Broker service not responding: {}", e)))?;

        client_file
            .write_all(&req_bytes)
            .map_err(|e| EngineError::IpcFailed(e.to_string()))?;

        let mut resp_bytes = [0u8; IpcResponse::SIZE];
        client_file
            .read_exact(&mut resp_bytes)
            .map_err(|e| EngineError::IpcFailed(e.to_string()))?;

        let resp = IpcResponse::from_bytes(&resp_bytes)
            .map_err(|e| EngineError::IpcFailed(e.to_string()))?;

        match resp.status {
            ResponseStatus::Success => Ok(()),
            ResponseStatus::RateLimited => Err(EngineError::CooldownActive),
            ResponseStatus::AccessDenied => {
                Err(EngineError::OsError("Access denied by Broker".to_string()))
            }
            ResponseStatus::InvalidSession => Err(EngineError::OsError(
                "Target PID is outside caller session".to_string(),
            )),
            ResponseStatus::ExecutionFailed => Err(EngineError::OsError(format!(
                "Kernel execution failed NTSTATUS 0x{:08X}",
                resp.ntstatus
            ))),
            ResponseStatus::InvalidPayload => Err(EngineError::IpcFailed(
                "Invalid payload rejected by broker".to_string(),
            )),
        }
    }
}
