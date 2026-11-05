//! Enumeration of active process memory consumers.

use crate::error::SysError;
use headroom_core::types::ProcessConsumer;
use std::mem::size_of;
use windows::Win32::Foundation::{CloseHandle, GetLastError};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

/// Collects list of running processes with memory usage, sorted descending by private bytes.
pub fn query_process_consumers(limit: usize) -> Result<Vec<ProcessConsumer>, SysError> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    // SAFETY: TH32CS_SNAPPROCESS with th32ProcessID = 0 captures snapshot of all system processes.
    let h_snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    let h_snap = match h_snap {
        Ok(h) => h,
        Err(_) => {
            let err = unsafe { GetLastError() };
            return Err(SysError::from_win32("CreateToolhelp32Snapshot", err.0));
        }
    };

    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };

    let mut consumers = Vec::new();

    // SAFETY: h_snap is a valid snapshot handle; entry is initialized with dwSize.
    let mut ok = unsafe { Process32FirstW(h_snap, &mut entry) }.is_ok();

    while ok {
        let pid = entry.th32ProcessID;

        // Skip System Idle (0) and System (4)
        if pid > 4 {
            let end_idx = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..end_idx]);

            // Attempt opening with PROCESS_QUERY_LIMITED_INFORMATION
            // SAFETY: Calling OpenProcess with read-only query right.
            let h_proc = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) };
            if let Ok(handle) = h_proc {
                let mut pmc: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
                let cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;

                // SAFETY: handle is valid; buffer size matches struct.
                let mem_res =
                    unsafe { GetProcessMemoryInfo(handle, &mut pmc as *mut _ as *mut _, cb) };

                if mem_res.is_ok() {
                    consumers.push(ProcessConsumer {
                        pid,
                        name,
                        private_bytes: pmc.PrivateUsage as u64,
                        working_set_bytes: pmc.WorkingSetSize as u64,
                        is_growing: false,
                    });
                }

                // SAFETY: Close process handle.
                unsafe {
                    let _ = CloseHandle(handle);
                }
            }
        }

        // SAFETY: Advance snapshot iterator.
        ok = unsafe { Process32NextW(h_snap, &mut entry) }.is_ok();
    }

    // SAFETY: Close snapshot handle.
    unsafe {
        let _ = CloseHandle(h_snap);
    }

    // Sort descending by private bytes
    consumers.sort_by_key(|a| std::cmp::Reverse(a.private_bytes));
    consumers.truncate(limit);

    Ok(consumers)
}
