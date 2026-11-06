//! Binary named pipe protocol for Headroom IPC.
//!
//! # Protocol Layout
//! All messages use fixed-size, versioned binary representations with no dynamic heap allocations.

use crate::error::IpcError;

/// Protocol magic header: ASCII 'HEAD' (0x48454144).
pub const IPC_MAGIC: u32 = 0x48454144;

/// Protocol schema version.
pub const IPC_VERSION: u8 = 1;

/// Named pipe identifier.
pub const PIPE_NAME: &str = r"\\.\pipe\HeadroomBroker";

/// Supported cleaning actions in increasing order of system disruption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ActionKind {
    /// Lower process memory priority (Opt-in).
    LowerMemoryPriority = 1,
    /// Trim working set of a specific process.
    TrimProcess = 2,
    /// Purge low-priority standby list pages (stutter profile).
    PurgeLowPriorityStandby = 3,
    /// Purge all standby list pages (with warning).
    PurgeStandby = 4,
    /// Flush modified list pages to disk (with warning).
    FlushModifiedList = 5,
    /// Empty all accessible process working sets (with warning).
    EmptyWorkingSets = 6,
}

impl ActionKind {
    /// Parses an action from a single raw byte.
    pub fn from_u8(val: u8) -> Result<Self, IpcError> {
        match val {
            1 => Ok(Self::LowerMemoryPriority),
            2 => Ok(Self::TrimProcess),
            3 => Ok(Self::PurgeLowPriorityStandby),
            4 => Ok(Self::PurgeStandby),
            5 => Ok(Self::FlushModifiedList),
            6 => Ok(Self::EmptyWorkingSets),
            _ => Err(IpcError::UnknownAction(val)),
        }
    }

    /// Whether this action requires a target process ID.
    pub fn requires_pid(&self) -> bool {
        matches!(self, Self::LowerMemoryPriority | Self::TrimProcess)
    }

    /// Human-readable label for the action.
    pub fn label(&self) -> &'static str {
        match self {
            Self::LowerMemoryPriority => "Lower Memory Priority",
            Self::TrimProcess => "Trim Process",
            Self::PurgeLowPriorityStandby => "Purge Low-Priority Standby",
            Self::PurgeStandby => "Purge All Standby",
            Self::FlushModifiedList => "Flush Modified List",
            Self::EmptyWorkingSets => "Empty Working Sets",
        }
    }
}

/// Request message sent from client to broker service.
/// Fixed size: 12 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcRequest {
    /// Protocol magic (must be IPC_MAGIC).
    pub magic: u32,
    /// Protocol version (must be IPC_VERSION).
    pub version: u8,
    /// Target action identifier.
    pub action: ActionKind,
    /// Reserved flags (for future extensions).
    pub flags: u16,
    /// Target process ID (0 if not applicable).
    pub target_pid: u32,
}

impl IpcRequest {
    pub const SIZE: usize = 12;

    /// Constructs a new IPC request.
    pub fn new(action: ActionKind, target_pid: Option<u32>) -> Self {
        Self {
            magic: IPC_MAGIC,
            version: IPC_VERSION,
            action,
            flags: 0,
            target_pid: target_pid.unwrap_or(0),
        }
    }

    /// Serializes request to a fixed 12-byte array.
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0..4].copy_from_slice(&self.magic.to_le_bytes());
        buf[4] = self.version;
        buf[5] = self.action as u8;
        buf[6..8].copy_from_slice(&self.flags.to_le_bytes());
        buf[8..12].copy_from_slice(&self.target_pid.to_le_bytes());
        buf
    }

    /// Deserializes and validates a request from a byte slice.
    pub fn from_bytes(slice: &[u8]) -> Result<Self, IpcError> {
        if slice.len() < Self::SIZE {
            return Err(IpcError::BufferTooShort {
                expected: Self::SIZE,
                actual: slice.len(),
            });
        }
        if slice.len() > Self::SIZE {
            return Err(IpcError::OversizedPayload(slice.len()));
        }

        let magic = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        if magic != IPC_MAGIC {
            return Err(IpcError::InvalidMagic(magic));
        }

        let version = slice[4];
        if version != IPC_VERSION {
            return Err(IpcError::UnsupportedVersion {
                found: version,
                supported: IPC_VERSION,
            });
        }

        let action = ActionKind::from_u8(slice[5])?;
        let flags = u16::from_le_bytes([slice[6], slice[7]]);
        let target_pid = u32::from_le_bytes([slice[8], slice[9], slice[10], slice[11]]);

        Ok(Self {
            magic,
            version,
            action,
            flags,
            target_pid,
        })
    }
}

/// Status code returned in broker response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ResponseStatus {
    /// Action succeeded.
    Success = 0,
    /// Action rejected due to rate limiting.
    RateLimited = 1,
    /// Caller lacks privilege or token session mismatch.
    AccessDenied = 2,
    /// Target PID is outside caller's session.
    InvalidSession = 3,
    /// Execution failed in OS kernel.
    ExecutionFailed = 4,
    /// Invalid payload format.
    InvalidPayload = 5,
}

impl ResponseStatus {
    pub fn from_u32(val: u32) -> Self {
        match val {
            0 => Self::Success,
            1 => Self::RateLimited,
            2 => Self::AccessDenied,
            3 => Self::InvalidSession,
            4 => Self::ExecutionFailed,
            _ => Self::InvalidPayload,
        }
    }
}

/// Response message sent from broker service to client.
/// Fixed size: 18 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcResponse {
    pub magic: u32,
    pub version: u8,
    pub status: ResponseStatus,
    pub ntstatus: u32,
    pub duration_us: u64,
}

impl IpcResponse {
    pub const SIZE: usize = 18;

    pub fn new(status: ResponseStatus, ntstatus: u32, duration_us: u64) -> Self {
        Self {
            magic: IPC_MAGIC,
            version: IPC_VERSION,
            status,
            ntstatus,
            duration_us,
        }
    }

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0..4].copy_from_slice(&self.magic.to_le_bytes());
        buf[4] = self.version;
        buf[5] = 0; // padding
        buf[6..10].copy_from_slice(&(self.status as u32).to_le_bytes());
        buf[10..14].copy_from_slice(&self.ntstatus.to_le_bytes());
        buf[14..22.min(Self::SIZE)]
            .copy_from_slice(&self.duration_us.to_le_bytes()[..Self::SIZE - 14]);
        buf
    }

    pub fn from_bytes(slice: &[u8]) -> Result<Self, IpcError> {
        if slice.len() < Self::SIZE {
            return Err(IpcError::BufferTooShort {
                expected: Self::SIZE,
                actual: slice.len(),
            });
        }

        let magic = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        if magic != IPC_MAGIC {
            return Err(IpcError::InvalidMagic(magic));
        }

        let version = slice[4];
        if version != IPC_VERSION {
            return Err(IpcError::UnsupportedVersion {
                found: version,
                supported: IPC_VERSION,
            });
        }

        let status_raw = u32::from_le_bytes([slice[6], slice[7], slice[8], slice[9]]);
        let ntstatus = u32::from_le_bytes([slice[10], slice[11], slice[12], slice[13]]);

        let mut dur_bytes = [0u8; 8];
        dur_bytes[..4].copy_from_slice(&slice[14..18]);
        let duration_us = u64::from_le_bytes(dur_bytes);

        Ok(Self {
            magic,
            version,
            status: ResponseStatus::from_u32(status_raw),
            ntstatus,
            duration_us,
        })
    }
}
