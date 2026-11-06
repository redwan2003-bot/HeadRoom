//! Fuzzing tests for IPC packet parser.
//!
//! Feeds random, boundary, truncated, and corrupted byte patterns into `IpcRequest::from_bytes`
//! and `IpcResponse::from_bytes` to verify robust error handling and zero panics.

use headroom_ipc::{ActionKind, IpcRequest, IpcResponse, ResponseStatus, IPC_MAGIC, IPC_VERSION};

#[test]
fn test_roundtrip_request() {
    let req = IpcRequest::new(ActionKind::TrimProcess, Some(1234));
    let bytes = req.to_bytes();
    let decoded = IpcRequest::from_bytes(&bytes).expect("Valid request must decode");
    assert_eq!(decoded.magic, IPC_MAGIC);
    assert_eq!(decoded.version, IPC_VERSION);
    assert_eq!(decoded.action, ActionKind::TrimProcess);
    assert_eq!(decoded.target_pid, 1234);
}

#[test]
fn test_roundtrip_response() {
    let resp = IpcResponse::new(ResponseStatus::Success, 0, 42000);
    let bytes = resp.to_bytes();
    let decoded = IpcResponse::from_bytes(&bytes).expect("Valid response must decode");
    assert_eq!(decoded.magic, IPC_MAGIC);
    assert_eq!(decoded.version, IPC_VERSION);
    assert_eq!(decoded.status, ResponseStatus::Success);
    assert_eq!(decoded.duration_us, 42000);
}

#[test]
fn test_fuzz_parser_no_panic() {
    // Deterministic pseudo-random sequence for fuzzing
    let mut state: u64 = 0xDEADBEEFCAFE;
    let mut next_rand = || -> u8 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (state >> 33) as u8
    };

    // Fuzz lengths from 0 to 64 bytes
    for len in 0..64 {
        for _ in 0..200 {
            let mut buf = vec![0u8; len];
            for b in &mut buf {
                *b = next_rand();
            }

            // Both parsers must return Result without panicking
            let _ = IpcRequest::from_bytes(&buf);
            let _ = IpcResponse::from_bytes(&buf);
        }
    }
}
