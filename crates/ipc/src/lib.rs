//! Headroom IPC: Binary protocol definition and frame validation.
//!
//! # Safety Invariant
//! `#![forbid(unsafe_code)]`. All message parsing is strictly bounds-checked.

#![forbid(unsafe_code)]

pub mod error;
pub mod protocol;

pub use error::IpcError;
pub use protocol::{
    ActionKind, IpcRequest, IpcResponse, ResponseStatus, IPC_MAGIC, IPC_VERSION, PIPE_NAME,
};
