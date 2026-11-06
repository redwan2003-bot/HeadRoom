//! Typed IPC protocol errors.

use thiserror::Error;

/// Errors occurring during IPC message encoding, decoding, and validation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum IpcError {
    /// Received buffer is too short to contain a valid message.
    #[error("Buffer too short: expected {expected} bytes, received {actual}")]
    BufferTooShort { expected: usize, actual: usize },

    /// Magic signature mismatch (expected 0x48454144 'HEAD').
    #[error("Invalid magic signature: 0x{0:08X}")]
    InvalidMagic(u32),

    /// Protocol version not supported.
    #[error("Unsupported protocol version: {found} (supported: {supported})")]
    UnsupportedVersion { found: u8, supported: u8 },

    /// Unknown action identifier.
    #[error("Unknown action ID: {0}")]
    UnknownAction(u8),

    /// Payload size exceeds fixed protocol bounds.
    #[error("Payload oversized: {0} bytes")]
    OversizedPayload(usize),
}
