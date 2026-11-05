//! Typed Windows and Performance Data Helper (PDH) errors.

use thiserror::Error;

/// System-level errors encountered when querying Windows counters and APIs.
#[derive(Debug, Error)]
pub enum SysError {
    /// Win32 API call failed with a specific error code.
    #[error("Win32 error: {function} failed with code 0x{code:08X} ({message})")]
    Win32 {
        function: &'static str,
        code: u32,
        message: String,
    },

    /// PDH counter operation failed.
    #[error("PDH counter error: {operation} returned code 0x{status:08X}")]
    Pdh {
        operation: &'static str,
        status: u32,
    },

    /// Snapshot or query resource unavailable.
    #[error("Resource unavailable: {0}")]
    Unavailable(&'static str),
}

impl SysError {
    pub(crate) fn from_win32(function: &'static str, code: u32) -> Self {
        Self::Win32 {
            function,
            code,
            message: format!("Error code {}", code),
        }
    }
}
