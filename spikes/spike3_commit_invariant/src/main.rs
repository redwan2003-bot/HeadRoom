//! Spike 3: VM Trimming vs. Commit Charge Invariant Probe
//!
//! SAFETY NOTICE:
//! To be executed inside an isolated Virtual Machine.
//! Empirically tests the invariant: Trimming does NOT lower commit charge.
//!
//! Workflow:
//! 1. Record baseline Working Set and System Commit.
//! 2. VirtualAlloc a known size (e.g. 128 MB) with MEM_COMMIT | MEM_RESERVE.
//! 3. Touch every page to force physical memory backing.
//! 4. Record post-allocation Working Set and System Commit.
//! 5. Call EmptyWorkingSet on the process.
//! 6. Record post-trim Working Set and System Commit.
//! 7. Output findings and delta verification.

use std::fs::File;
use std::io::Write;
use std::mem::size_of;
use windows::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
use windows::Win32::System::ProcessStatus::{
    EmptyWorkingSet, GetPerformanceInfo, GetProcessMemoryInfo, PERFORMANCE_INFORMATION,
    PROCESS_MEMORY_COUNTERS_EX,
};
use windows::Win32::System::Threading::GetCurrentProcess;

fn get_process_working_set_kb() -> usize {
    unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
        let cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        let _ = GetProcessMemoryInfo(GetCurrentProcess(), &mut counters as *mut _ as *mut _, cb);
        counters.WorkingSetSize / 1024
    }
}

fn get_system_commit_kb() -> usize {
    unsafe {
        let mut perf_info: PERFORMANCE_INFORMATION = std::mem::zeroed();
        let cb = size_of::<PERFORMANCE_INFORMATION>() as u32;
        perf_info.cb = cb;
        let _ = GetPerformanceInfo(&mut perf_info, cb);
        (perf_info.CommitTotal * perf_info.PageSize) / 1024
    }
}

fn main() {
    println!("Headroom Spike 3: Trimming vs Commit Invariant Probe\n");

    let alloc_size_bytes = 128 * 1024 * 1024; // 128 MB
    let alloc_size_kb = alloc_size_bytes / 1024;

    // Phase 1: Baseline
    let ws_baseline = get_process_working_set_kb();
    let commit_baseline = get_system_commit_kb();
    println!("[1] Baseline:");
    println!("    Process Working Set: {} KB", ws_baseline);
    println!("    System Commit Total: {} KB", commit_baseline);

    // Phase 2: Allocate & Touch
    println!("\n[2] Allocating and touching 128 MB...");
    let ptr = unsafe {
        VirtualAlloc(
            None,
            alloc_size_bytes,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        )
    };
    assert!(!ptr.is_null(), "VirtualAlloc failed");

    // Dirty every 4096-byte page
    let slice = unsafe { std::slice::from_raw_parts_mut(ptr as *mut u8, alloc_size_bytes) };
    for page in slice.chunks_mut(4096) {
        page[0] = 0x5A;
    }

    let ws_post_alloc = get_process_working_set_kb();
    let commit_post_alloc = get_system_commit_kb();
    println!(
        "    Process Working Set post-alloc: {} KB (delta: +{} KB)",
        ws_post_alloc,
        ws_post_alloc.saturating_sub(ws_baseline)
    );
    println!(
        "    System Commit post-alloc:       {} KB (delta: +{} KB)",
        commit_post_alloc,
        commit_post_alloc.saturating_sub(commit_baseline)
    );

    // Phase 3: EmptyWorkingSet (Trim)
    println!("\n[3] Calling EmptyWorkingSet on current process...");
    let trim_res = unsafe { EmptyWorkingSet(GetCurrentProcess()) };
    assert!(trim_res.is_ok(), "EmptyWorkingSet failed");

    let ws_post_trim = get_process_working_set_kb();
    let commit_post_trim = get_system_commit_kb();
    println!(
        "    Process Working Set post-trim:  {} KB (delta from alloc: -{} KB)",
        ws_post_trim,
        ws_post_alloc.saturating_sub(ws_post_trim)
    );
    println!(
        "    System Commit post-trim:        {} KB (delta from alloc: {} KB)",
        commit_post_trim,
        (commit_post_trim as i64) - (commit_post_alloc as i64)
    );

    // Cleanup allocation
    unsafe {
        let _ = VirtualFree(ptr, 0, MEM_RELEASE);
    }

    let commit_delta = (commit_post_trim as i64) - (commit_post_alloc as i64);
    let ws_drop = ws_post_alloc.saturating_sub(ws_post_trim);
    let commit_invariant_holds = commit_delta.abs() < 5120; // within 5 MB OS variance

    println!("\n=== Analysis ===");
    println!(
        "Working Set dropped: {} KB (~{:.1} MB)",
        ws_drop,
        ws_drop as f64 / 1024.0
    );
    println!("Commit delta:        {} KB", commit_delta);
    println!(
        "Invariant 'Trimming does NOT lower commit charge' holds: {}",
        commit_invariant_holds
    );

    let json_output = format!(
        r#"{{
  "alloc_kb": {},
  "ws_baseline_kb": {},
  "ws_post_alloc_kb": {},
  "ws_post_trim_kb": {},
  "commit_baseline_kb": {},
  "commit_post_alloc_kb": {},
  "commit_post_trim_kb": {},
  "commit_delta_kb": {},
  "invariant_holds": {}
}}"#,
        alloc_size_kb,
        ws_baseline,
        ws_post_alloc,
        ws_post_trim,
        commit_baseline,
        commit_post_alloc,
        commit_post_trim,
        commit_delta,
        commit_invariant_holds
    );

    let _ = std::fs::create_dir_all("results");
    if let Ok(mut f) = File::create("results/spike3.json") {
        let _ = f.write_all(json_output.as_bytes());
        println!("\n[+] Saved results to results/spike3.json");
    }
}
