//! Headroom Privileged Broker Service.
//!
//! # Safety Notice
//! Designed to run as a Windows Service with SeProfileSingleProcessPrivilege.
//! Must only be executed / tested inside a Virtual Machine.

use headroom_ipc::{ActionKind, IpcRequest, IpcResponse, ResponseStatus, PIPE_NAME};
use headroom_sys::undocumented::{execute_memory_list_command, MemoryListCommand};
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, LUID};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::Storage::FileSystem::{
    ReadFile, WriteFile, FILE_FLAGS_AND_ATTRIBUTES, FILE_FLAG_FIRST_PIPE_INSTANCE,
    PIPE_ACCESS_DUPLEX,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_MESSAGE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_MESSAGE, PIPE_UNLIMITED_INSTANCES,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

static RUNNING: AtomicBool = AtomicBool::new(true);

fn enable_privilege(name: PCWSTR) -> bool {
    // SAFETY: OpenProcessToken with TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY to adjust process privileges.
    unsafe {
        let mut h_token = HANDLE::default();
        if OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut h_token,
        )
        .is_err()
        {
            return false;
        }

        let mut luid = LUID::default();
        if LookupPrivilegeValueW(PCWSTR::null(), name, &mut luid).is_err() {
            let _ = CloseHandle(h_token);
            return false;
        }

        let tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };

        let res = AdjustTokenPrivileges(
            h_token,
            false,
            Some(&tp),
            size_of::<TOKEN_PRIVILEGES>() as u32,
            None,
            None,
        );

        let err = GetLastError();
        let _ = CloseHandle(h_token);
        res.is_ok() && err.0 == 0
    }
}

fn handle_client(h_pipe: HANDLE, last_purge: &mut Instant) {
    let mut req_bytes = [0u8; IpcRequest::SIZE];
    let mut bytes_read = 0u32;

    // SAFETY: h_pipe is a connected named pipe handle; req_bytes is a valid buffer of IpcRequest::SIZE.
    unsafe {
        let read_res = ReadFile(h_pipe, Some(&mut req_bytes), Some(&mut bytes_read), None);

        if read_res.is_err() || bytes_read as usize != IpcRequest::SIZE {
            let _ = DisconnectNamedPipe(h_pipe);
            return;
        }
    }

    let req = match IpcRequest::from_bytes(&req_bytes) {
        Ok(r) => r,
        Err(_) => {
            send_response(h_pipe, ResponseStatus::InvalidPayload, 0, 0);
            return;
        }
    };

    // Rate-limiting check for destructive purges (minimum 15s between broker purges)
    let is_destructive = matches!(
        req.action,
        ActionKind::PurgeStandby | ActionKind::FlushModifiedList | ActionKind::EmptyWorkingSets
    );

    if is_destructive && last_purge.elapsed() < Duration::from_secs(15) {
        send_response(h_pipe, ResponseStatus::RateLimited, 0, 0);
        return;
    }

    let start = Instant::now();
    let result = match req.action {
        ActionKind::PurgeLowPriorityStandby => {
            execute_memory_list_command(MemoryListCommand::PurgeLowPriorityStandbyList)
        }
        ActionKind::PurgeStandby => {
            *last_purge = Instant::now();
            execute_memory_list_command(MemoryListCommand::PurgeStandbyList)
        }
        ActionKind::FlushModifiedList => {
            *last_purge = Instant::now();
            execute_memory_list_command(MemoryListCommand::FlushModifiedList)
        }
        ActionKind::EmptyWorkingSets => {
            *last_purge = Instant::now();
            execute_memory_list_command(MemoryListCommand::EmptyWorkingSets)
        }
        _ => {
            // Unprivileged actions are handled by agent
            Ok(0)
        }
    };

    let duration_us = start.elapsed().as_micros() as u64;

    match result {
        Ok(_) => send_response(h_pipe, ResponseStatus::Success, 0, duration_us),
        Err(_) => send_response(
            h_pipe,
            ResponseStatus::ExecutionFailed,
            0xC0000001,
            duration_us,
        ),
    }
}

fn send_response(h_pipe: HANDLE, status: ResponseStatus, ntstatus: u32, duration_us: u64) {
    let resp = IpcResponse::new(status, ntstatus, duration_us);
    let bytes = resp.to_bytes();
    let mut written = 0u32;
    // SAFETY: h_pipe is a valid named pipe handle; bytes is a stack buffer of IpcResponse::SIZE.
    unsafe {
        let _ = WriteFile(h_pipe, Some(&bytes), Some(&mut written), None);
        let _ = DisconnectNamedPipe(h_pipe);
    }
}

fn run_broker_loop() {
    println!("[*] Starting Headroom Broker server on {}...", PIPE_NAME);

    // Enable SeProfileSingleProcessPrivilege
    let priv_ok = enable_privilege(w!("SeProfileSingleProcessPrivilege"));
    println!("[*] SeProfileSingleProcessPrivilege enabled: {}", priv_ok);

    let pipe_wide: Vec<u16> = PIPE_NAME.encode_utf16().chain(std::iter::once(0)).collect();
    let mut last_purge = Instant::now() - Duration::from_secs(60);

    let mut is_first = true;

    while RUNNING.load(Ordering::Relaxed) {
        let open_mode = PIPE_ACCESS_DUPLEX
            | if is_first {
                FILE_FLAG_FIRST_PIPE_INSTANCE
            } else {
                FILE_FLAGS_AND_ATTRIBUTES(0)
            };
        let pipe_mode = PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_REJECT_REMOTE_CLIENTS;

        // SAFETY: Calling CreateNamedPipeW with explicit flags, non-remote restriction, message mode.
        let h_pipe = unsafe {
            CreateNamedPipeW(
                PCWSTR::from_raw(pipe_wide.as_ptr()),
                open_mode,
                pipe_mode,
                PIPE_UNLIMITED_INSTANCES,
                1024,
                1024,
                0,
                None,
            )
        };

        if h_pipe.is_invalid() {
            eprintln!("[-] CreateNamedPipe failed: {:?}", unsafe {
                GetLastError()
            });
            std::thread::sleep(Duration::from_millis(1000));
            continue;
        }

        let handle = h_pipe;
        is_first = false;

        // SAFETY: handle is a valid pipe handle.
        let connected = unsafe { ConnectNamedPipe(handle, None) };
        if connected.is_ok() || unsafe { GetLastError() }.0 == 535 {
            // ERROR_PIPE_CONNECTED
            handle_client(handle, &mut last_purge);
        }

        // SAFETY: Close pipe instance handle.
        unsafe {
            let _ = CloseHandle(handle);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--console") {
        println!("Headroom Broker (Console Test Mode for VM)");
        run_broker_loop();
    } else {
        // Run standalone loop
        run_broker_loop();
    }
}
