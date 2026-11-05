//! Spike 1: Testing unelevated memory counter collection via PDH, GetPerformanceInfo, and NtQuerySystemInformation.
//!
//! Evaluates whether an unelevated process can retrieve:
//! - Available, Modified, Free & Zero, Standby (by priority) list sizes
//! - Commit charge & limit
//! - Hard-fault / Pages Input rate per second

use std::mem::{size_of, zeroed};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{ERROR_SUCCESS, NTSTATUS};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterValue,
    PdhOpenQueryW, PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_FMT_LARGE,
};
use windows::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct SystemMemoryListInformation {
    zero_page_count: usize,
    free_page_count: usize,
    modified_page_count: usize,
    modified_no_write_page_count: usize,
    bad_page_count: usize,
    page_bits: [usize; 8],
    standby_page_count: [usize; 8],
}

type NtQuerySystemInformationFn = unsafe extern "system" fn(
    system_information_class: u32,
    system_information: *mut std::ffi::c_void,
    system_information_length: u32,
    return_length: *mut u32,
) -> NTSTATUS;

fn test_performance_info() {
    println!("=== 1. GetPerformanceInfo (Documented PSAPI) ===");
    let mut perf_info: PERFORMANCE_INFORMATION = unsafe { zeroed() };
    let cb = size_of::<PERFORMANCE_INFORMATION>() as u32;
    perf_info.cb = cb;

    let res = unsafe { GetPerformanceInfo(&mut perf_info, cb) };
    if res.is_err() {
        eprintln!("[-] GetPerformanceInfo failed: {:?}", res);
        return;
    }

    let page_size_kb = perf_info.PageSize / 1024;
    println!(
        "[+] CommitTotal: {} MB",
        (perf_info.CommitTotal * perf_info.PageSize) / (1024 * 1024)
    );
    println!(
        "[+] CommitLimit: {} MB",
        (perf_info.CommitLimit * perf_info.PageSize) / (1024 * 1024)
    );
    println!(
        "[+] CommitPeak:  {} MB",
        (perf_info.CommitPeak * perf_info.PageSize) / (1024 * 1024)
    );
    println!(
        "[+] PhysicalTotal: {} MB",
        (perf_info.PhysicalTotal * perf_info.PageSize) / (1024 * 1024)
    );
    println!(
        "[+] PhysicalAvailable: {} MB",
        (perf_info.PhysicalAvailable * perf_info.PageSize) / (1024 * 1024)
    );
    println!(
        "[+] KernelTotal: {} MB",
        (perf_info.KernelTotal * perf_info.PageSize) / (1024 * 1024)
    );
    println!("[+] PageSize: {} KB", page_size_kb);
}

fn test_pdh_counters() {
    println!("\n=== 2. PDH English Counters (PdhAddEnglishCounterW) ===");

    let mut query: isize = 0;
    let status = unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) };
    if status != ERROR_SUCCESS.0 {
        eprintln!("[-] PdhOpenQueryW failed with code: 0x{:08X}", status);
        return;
    }

    let counter_paths = [
        (r"\Memory\Available Bytes", true),
        (r"\Memory\Modified Page List Bytes", true),
        (r"\Memory\Free & Zero Page List Bytes", true),
        (r"\Memory\Standby Cache Reserve Bytes", true),
        (r"\Memory\Standby Cache Normal Priority Bytes", true),
        (r"\Memory\Standby Cache Core Bytes", true),
        (r"\Memory\Standby Cache 0 Priority Bytes", true),
        (r"\Memory\Standby Cache 1 Priority Bytes", true),
        (r"\Memory\Standby Cache 2 Priority Bytes", true),
        (r"\Memory\Standby Cache 3 Priority Bytes", true),
        (r"\Memory\Standby Cache 4 Priority Bytes", true),
        (r"\Memory\Standby Cache 5 Priority Bytes", true),
        (r"\Memory\Standby Cache 6 Priority Bytes", true),
        (r"\Memory\Standby Cache 7 Priority Bytes", true),
        (r"\Memory\Pages Input/sec", false),
    ];

    let mut handles: Vec<(&str, bool, isize)> = Vec::new();

    for &(path, is_large) in &counter_paths {
        let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let mut counter_handle: isize = 0;
        let add_res = unsafe {
            PdhAddEnglishCounterW(
                query,
                PCWSTR::from_raw(wide_path.as_ptr()),
                0,
                &mut counter_handle,
            )
        };

        if add_res == ERROR_SUCCESS.0 {
            handles.push((path, is_large, counter_handle));
            println!("[+] Counter added successfully: {}", path);
        } else {
            eprintln!("[-] Counter failed: {} (code: 0x{:08X})", path, add_res);
        }
    }

    // Collect first sample
    let coll1 = unsafe { PdhCollectQueryData(query) };
    println!("[*] Initial sample collection status: 0x{:08X}", coll1);

    // Sleep 1 second for rate counters (e.g. Pages Input/sec)
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let coll2 = unsafe { PdhCollectQueryData(query) };
    println!("[*] Second sample collection status: 0x{:08X}", coll2);

    for &(path, is_large, handle) in &handles {
        let mut val: PDH_FMT_COUNTERVALUE = unsafe { zeroed() };
        let format_flag = if is_large {
            PDH_FMT_LARGE
        } else {
            PDH_FMT_DOUBLE
        };
        let fmt_res = unsafe { PdhGetFormattedCounterValue(handle, format_flag, None, &mut val) };

        if fmt_res == ERROR_SUCCESS.0 {
            unsafe {
                if is_large {
                    let bytes = val.Anonymous.largeValue;
                    println!(
                        "    {} = {} MB ({} bytes)",
                        path,
                        bytes / (1024 * 1024),
                        bytes
                    );
                } else {
                    let rate = val.Anonymous.doubleValue;
                    println!("    {} = {:.2}/sec", path, rate);
                }
            }
        } else {
            eprintln!("[-] Format failed for {}: 0x{:08X}", path, fmt_res);
        }
    }

    unsafe {
        let _ = PdhCloseQuery(query);
    }
}

fn test_nt_query_system_information() {
    println!("\n=== 3. NtQuerySystemInformation (SystemMemoryListInformation = 80) ===");
    unsafe {
        let ntdll = GetModuleHandleW(w!("ntdll.dll"));
        if ntdll.is_err() {
            eprintln!("[-] Failed to get ntdll.dll handle");
            return;
        }

        let proc = GetProcAddress(
            ntdll.unwrap(),
            windows::core::s!("NtQuerySystemInformation"),
        );
        if proc.is_none() {
            eprintln!("[-] NtQuerySystemInformation symbol not found");
            return;
        }

        let nt_query: NtQuerySystemInformationFn = std::mem::transmute(proc);
        let mut mem_info: SystemMemoryListInformation = zeroed();
        let mut return_len: u32 = 0;

        let status = nt_query(
            80, // SystemMemoryListInformation
            &mut mem_info as *mut _ as *mut std::ffi::c_void,
            size_of::<SystemMemoryListInformation>() as u32,
            &mut return_len,
        );

        println!(
            "[*] NtQuerySystemInformation(80) NTSTATUS: 0x{:08X}",
            status.0
        );
        if status.0 == 0 {
            println!("[+] Successfully queried unelevated!");
            println!("    ZeroPageCount:     {}", mem_info.zero_page_count);
            println!("    FreePageCount:     {}", mem_info.free_page_count);
            println!("    ModifiedPageCount: {}", mem_info.modified_page_count);
            println!("    StandbyPageCounts: {:?}", mem_info.standby_page_count);
        } else {
            println!("[-] Query rejected (likely requires elevation or SeProfileSingleProcessPrivilege). NTSTATUS: 0x{:08X}", status.0);
        }
    }
}

fn main() {
    println!("Headroom Spike 1: Unelevated System Memory Inspection\n");
    test_performance_info();
    test_pdh_counters();
    test_nt_query_system_information();
}
