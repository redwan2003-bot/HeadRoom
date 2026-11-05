//! Spike 5: Verify whether an unelevated process can trim and set memory priority on its own user's processes.
//!
//! Investigates:
//! - Exact handle access rights required for EmptyWorkingSet (PROCESS_SET_QUOTA vs PROCESS_QUERY_LIMITED_INFORMATION)
//! - Exact handle access rights for SetProcessInformation(ProcessMemoryPriority)
//! - Verifies behavior on current process and a spawned child process
//! - Confirms that access is denied on processes belonging to other users/system

use std::mem::size_of;
use std::process::Command;
use windows::Win32::Foundation::{CloseHandle, GetLastError};
use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetProcessInformation, OpenProcess, ProcessMemoryPriority,
    SetProcessInformation, MEMORY_PRIORITY_INFORMATION, MEMORY_PRIORITY_LOW,
    MEMORY_PRIORITY_NORMAL, MEMORY_PRIORITY_VERY_LOW, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SET_INFORMATION, PROCESS_SET_QUOTA,
};

fn test_current_process() {
    println!("=== 1. Current Process Tests ===");
    let h_current = unsafe { GetCurrentProcess() };

    // 1a. Test EmptyWorkingSet on self
    let trim_res = unsafe { EmptyWorkingSet(h_current) };
    if trim_res.is_ok() {
        println!("[+] EmptyWorkingSet on self: SUCCESS");
    } else {
        println!("[-] EmptyWorkingSet on self failed: {:?}", unsafe {
            GetLastError()
        });
    }

    // 1b. Test SetProcessInformation on self (Lower memory priority)
    let mut mem_prio = MEMORY_PRIORITY_INFORMATION {
        MemoryPriority: MEMORY_PRIORITY_VERY_LOW,
    };
    let set_res = unsafe {
        SetProcessInformation(
            h_current,
            ProcessMemoryPriority,
            &mem_prio as *const _ as *const std::ffi::c_void,
            size_of::<MEMORY_PRIORITY_INFORMATION>() as u32,
        )
    };
    if set_res.is_ok() {
        println!("[+] SetProcessInformation(MEMORY_PRIORITY_VERY_LOW) on self: SUCCESS");
    } else {
        println!("[-] SetProcessInformation failed on self: {:?}", unsafe {
            GetLastError()
        });
    }

    // 1c. Verify with GetProcessInformation
    let mut read_prio = MEMORY_PRIORITY_INFORMATION::default();
    let get_res = unsafe {
        GetProcessInformation(
            h_current,
            ProcessMemoryPriority,
            &mut read_prio as *mut _ as *mut std::ffi::c_void,
            size_of::<MEMORY_PRIORITY_INFORMATION>() as u32,
        )
    };
    if get_res.is_ok() {
        println!(
            "[+] Verified Memory Priority: {:?}",
            read_prio.MemoryPriority
        );
    } else {
        println!("[-] GetProcessInformation failed on self: {:?}", unsafe {
            GetLastError()
        });
    }

    // Restore normal
    mem_prio.MemoryPriority = MEMORY_PRIORITY_NORMAL;
    let _ = unsafe {
        SetProcessInformation(
            h_current,
            ProcessMemoryPriority,
            &mem_prio as *const _ as *const std::ffi::c_void,
            size_of::<MEMORY_PRIORITY_INFORMATION>() as u32,
        )
    };
}

fn test_child_process() {
    println!("\n=== 2. Same-User Child Process Tests ===");
    // Spawn a long-lived sleeping child process
    let mut child = Command::new("powershell")
        .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 15"])
        .spawn()
        .expect("Failed to spawn child process");

    let pid = child.id();
    println!("[*] Spawned child powershell.exe with PID: {}", pid);

    // Test Handle Right Combinations:
    // Case A: PROCESS_SET_QUOTA only
    unsafe {
        println!("\n[*] Testing OpenProcess with PROCESS_SET_QUOTA only:");
        let h_proc = OpenProcess(PROCESS_SET_QUOTA, false, pid);
        match h_proc {
            Ok(handle) => {
                println!("[+] OpenProcess(PROCESS_SET_QUOTA) succeeded");
                let trim_res = EmptyWorkingSet(handle);
                println!("    EmptyWorkingSet result: {:?}", trim_res);
                let _ = CloseHandle(handle);
            }
            Err(e) => println!("[-] OpenProcess(PROCESS_SET_QUOTA) failed: {:?}", e),
        }
    }

    // Case B: PROCESS_SET_INFORMATION only
    unsafe {
        println!("\n[*] Testing OpenProcess with PROCESS_SET_INFORMATION only:");
        let h_proc = OpenProcess(PROCESS_SET_INFORMATION, false, pid);
        match h_proc {
            Ok(handle) => {
                println!("[+] OpenProcess(PROCESS_SET_INFORMATION) succeeded");
                let mem_prio = MEMORY_PRIORITY_INFORMATION {
                    MemoryPriority: MEMORY_PRIORITY_LOW,
                };
                let set_res = SetProcessInformation(
                    handle,
                    ProcessMemoryPriority,
                    &mem_prio as *const _ as *const std::ffi::c_void,
                    size_of::<MEMORY_PRIORITY_INFORMATION>() as u32,
                );
                println!("    SetProcessInformation result: {:?}", set_res);
                let _ = CloseHandle(handle);
            }
            Err(e) => println!("[-] OpenProcess(PROCESS_SET_INFORMATION) failed: {:?}", e),
        }
    }

    // Case C: Combined rights (PROCESS_SET_QUOTA | PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION)
    unsafe {
        let combined_rights =
            PROCESS_SET_QUOTA | PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION;
        println!("\n[*] Testing OpenProcess with combined rights (SET_QUOTA | SET_INFORMATION | QUERY_LIMITED_INFORMATION):");
        let h_proc = OpenProcess(combined_rights, false, pid);
        match h_proc {
            Ok(handle) => {
                println!("[+] OpenProcess(combined) succeeded");
                let trim_res = EmptyWorkingSet(handle);
                println!("    EmptyWorkingSet: {:?}", trim_res);

                let mem_prio = MEMORY_PRIORITY_INFORMATION {
                    MemoryPriority: MEMORY_PRIORITY_LOW,
                };
                let set_res = SetProcessInformation(
                    handle,
                    ProcessMemoryPriority,
                    &mem_prio as *const _ as *const std::ffi::c_void,
                    size_of::<MEMORY_PRIORITY_INFORMATION>() as u32,
                );
                println!("    SetProcessInformation: {:?}", set_res);

                let mut verify_prio = MEMORY_PRIORITY_INFORMATION::default();
                let get_res = GetProcessInformation(
                    handle,
                    ProcessMemoryPriority,
                    &mut verify_prio as *mut _ as *mut std::ffi::c_void,
                    size_of::<MEMORY_PRIORITY_INFORMATION>() as u32,
                );
                println!(
                    "    GetProcessInformation: {:?} (Priority = {:?})",
                    get_res, verify_prio.MemoryPriority
                );
                let _ = CloseHandle(handle);
            }
            Err(e) => println!("[-] OpenProcess(combined) failed: {:?}", e),
        }
    }

    let _ = child.kill();
    let _ = child.wait();
}

fn test_system_process() {
    println!("\n=== 3. Privileged / System Process Isolation Test ===");
    // Attempt opening PID 4 (System) with PROCESS_SET_QUOTA
    unsafe {
        let h_proc = OpenProcess(PROCESS_SET_QUOTA, false, 4);
        match h_proc {
            Ok(handle) => {
                println!("[-] UNEXPECTED: Opened PID 4 (System)!");
                let _ = CloseHandle(handle);
            }
            Err(e) => {
                println!("[+] Expected Security Rejection on PID 4: {:?}", e);
            }
        }
    }
}

fn main() {
    println!("Headroom Spike 5: Unelevated Trimming & Memory Priority Verification\n");
    test_current_process();
    test_child_process();
    test_system_process();
}
