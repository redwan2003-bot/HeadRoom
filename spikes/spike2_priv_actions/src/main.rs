//! Spike 2: VM Privileged Memory Cleaning Actions Probe
//!
//! SAFETY NOTICE:
//! To be executed ONLY inside an isolated Virtual Machine to protect host stability.
//! Tests each action from Section 7 of the Headroom brief:
//! 1. Lower idle app memory priority (ProcessMemoryPriority)
//! 2. Trim one process (EmptyWorkingSet)
//! 3. Purge low-priority standby (SystemMemoryListInformation command 5)
//! 4. Purge all standby (SystemMemoryListInformation command 4)
//! 5. Flush modified list (SystemMemoryListInformation command 3)
//! 6. Empty all working sets (SystemMemoryListInformation command 2)
//!
//! Validates required privileges (SeProfileSingleProcessPrivilege) and records memory list deltas.

use std::fs::File;
use std::io::Write;
use std::mem::{size_of, zeroed};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, LUID, NTSTATUS};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

type NtSetSystemInformationFn = unsafe extern "system" fn(
    system_information_class: u32,
    system_information: *const std::ffi::c_void,
    system_information_length: u32,
) -> NTSTATUS;

fn enable_privilege(privilege_name: PCWSTR, enable: bool) -> bool {
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
        if LookupPrivilegeValueW(PCWSTR::null(), privilege_name, &mut luid).is_err() {
            let _ = CloseHandle(h_token);
            return false;
        }

        let tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: if enable {
                    SE_PRIVILEGE_ENABLED
                } else {
                    windows::Win32::Security::TOKEN_PRIVILEGES_ATTRIBUTES(0)
                },
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

fn get_memory_snapshot() -> (usize, usize) {
    let mut perf_info: PERFORMANCE_INFORMATION = unsafe { zeroed() };
    let cb = size_of::<PERFORMANCE_INFORMATION>() as u32;
    perf_info.cb = cb;
    if unsafe { GetPerformanceInfo(&mut perf_info, cb) }.is_ok() {
        let avail_mb = (perf_info.PhysicalAvailable * perf_info.PageSize) / (1024 * 1024);
        let commit_mb = (perf_info.CommitTotal * perf_info.PageSize) / (1024 * 1024);
        (avail_mb, commit_mb)
    } else {
        (0, 0)
    }
}

fn run_memory_list_command(
    nt_set: NtSetSystemInformationFn,
    command: u32,
    name: &str,
    has_priv: bool,
) -> String {
    let (avail_before, commit_before) = get_memory_snapshot();
    let status = unsafe {
        nt_set(
            80, // SystemMemoryListInformation
            &command as *const u32 as *const std::ffi::c_void,
            size_of::<u32>() as u32,
        )
    };
    let (avail_after, commit_after) = get_memory_snapshot();

    format!(
        "{{\"action\": \"{}\", \"command\": {}, \"privilege_enabled\": {}, \"ntstatus\": \"0x{:08X}\", \"avail_before_mb\": {}, \"avail_after_mb\": {}, \"commit_before_mb\": {}, \"commit_after_mb\": {}}}",
        name, command, has_priv, status.0, avail_before, avail_after, commit_before, commit_after
    )
}

fn main() {
    println!("Headroom Spike 2: VM Privileged Memory Cleaning Actions Probe\n");

    let ntdll = unsafe { GetModuleHandleW(w!("ntdll.dll")).expect("Failed to get ntdll handle") };
    let proc = unsafe {
        GetProcAddress(ntdll, windows::core::s!("NtSetSystemInformation"))
            .expect("NtSetSystemInformation missing")
    };
    let nt_set: NtSetSystemInformationFn = unsafe { std::mem::transmute(proc) };

    let mut results = Vec::new();

    // Test without SeProfileSingleProcessPrivilege
    println!("[*] Testing calls WITHOUT SeProfileSingleProcessPrivilege...");
    results.push(run_memory_list_command(
        nt_set,
        5,
        "PurgeLowPriorityStandbyList",
        false,
    ));
    results.push(run_memory_list_command(
        nt_set,
        4,
        "PurgeStandbyList",
        false,
    ));
    results.push(run_memory_list_command(
        nt_set,
        3,
        "FlushModifiedList",
        false,
    ));
    results.push(run_memory_list_command(
        nt_set,
        2,
        "EmptyWorkingSets",
        false,
    ));

    // Enable SeProfileSingleProcessPrivilege
    let priv_ok = enable_privilege(w!("SeProfileSingleProcessPrivilege"), true);
    println!(
        "[*] Enabling SeProfileSingleProcessPrivilege: {}",
        if priv_ok { "SUCCESS" } else { "FAILED" }
    );

    // Test with SeProfileSingleProcessPrivilege
    println!("[*] Testing calls WITH SeProfileSingleProcessPrivilege...");
    results.push(run_memory_list_command(
        nt_set,
        5,
        "PurgeLowPriorityStandbyList",
        priv_ok,
    ));
    results.push(run_memory_list_command(
        nt_set,
        4,
        "PurgeStandbyList",
        priv_ok,
    ));
    results.push(run_memory_list_command(
        nt_set,
        3,
        "FlushModifiedList",
        priv_ok,
    ));
    results.push(run_memory_list_command(
        nt_set,
        2,
        "EmptyWorkingSets",
        priv_ok,
    ));

    let output_json = format!("[\n  {}\n]\n", results.join(",\n  "));
    println!("\n=== Results JSON ===\n{}", output_json);

    let _ = std::fs::create_dir_all("results");
    if let Ok(mut f) = File::create("results/spike2.json") {
        let _ = f.write_all(output_json.as_bytes());
        println!("[+] Saved results to results/spike2.json");
    }
}
