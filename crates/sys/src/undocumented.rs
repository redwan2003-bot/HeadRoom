//! Quarantined undocumented Windows NT system calls.
//!
//! # Soundness Invariant
//! All undocumented calls live strictly in this file according to Section 4 of the engineering brief.
//! Functions are resolved dynamically at runtime via `GetProcAddress` so that execution fails
//! gracefully on builds where the API is unavailable or signature differs.

use crate::error::SysError;
use std::mem::size_of;
use windows::core::{s, w};
use windows::Win32::Foundation::NTSTATUS;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

type NtSetSystemInformationFn = unsafe extern "system" fn(
    system_information_class: u32,
    system_information: *const std::ffi::c_void,
    system_information_length: u32,
) -> NTSTATUS;

/// Information class 80: SystemMemoryListInformation.
const SYSTEM_MEMORY_LIST_INFORMATION: u32 = 80;

/// Memory list commands supported by SystemMemoryListInformation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MemoryListCommand {
    EmptyWorkingSets = 2,
    FlushModifiedList = 3,
    PurgeStandbyList = 4,
    PurgeLowPriorityStandbyList = 5,
}

/// Dynamically resolves and invokes NtSetSystemInformation with SystemMemoryListInformation (80).
///
/// Requires `SeProfileSingleProcessPrivilege` enabled in the calling process token.
pub fn execute_memory_list_command(command: MemoryListCommand) -> Result<u32, SysError> {
    let cmd_raw = command as u32;

    // SAFETY: GetModuleHandleW is called for ntdll.dll which is always loaded in every Win32 process.
    let h_ntdll = unsafe { GetModuleHandleW(w!("ntdll.dll")) };
    let h_ntdll = match h_ntdll {
        Ok(h) => h,
        Err(_) => return Err(SysError::Unavailable("ntdll.dll handle unavailable")),
    };

    // SAFETY: Resolving undocumented symbol NtSetSystemInformation dynamically.
    let proc = unsafe { GetProcAddress(h_ntdll, s!("NtSetSystemInformation")) };
    let proc = match proc {
        Some(p) => p,
        None => return Err(SysError::Unavailable("NtSetSystemInformation not found")),
    };

    // SAFETY: Transmuting to NtSetSystemInformationFn signature confirmed by Geoff Chappell & Process Hacker docs.
    let nt_set: NtSetSystemInformationFn = unsafe { std::mem::transmute(proc) };

    // SAFETY: Invoking NtSetSystemInformation with valid pointer to u32 command code and correct length.
    let status = unsafe {
        nt_set(
            SYSTEM_MEMORY_LIST_INFORMATION,
            &cmd_raw as *const u32 as *const std::ffi::c_void,
            size_of::<u32>() as u32,
        )
    };

    if status.0 == 0 {
        Ok(0)
    } else {
        Err(SysError::Win32 {
            function: "NtSetSystemInformation(80)",
            code: status.0 as u32,
            message: format!("NTSTATUS 0x{:08X}", status.0),
        })
    }
}
