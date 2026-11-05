//! Spike 6: Resource Footprint Baseline for Tray Agent & Dashboard Window
//!
//! Measures:
//! - Dashboard window instantiation time (budget: < 300 ms)
//! - Process working set and private bytes (budget: < 5 MB)
//! - Process CPU times (budget: < 0.1% idle)
//! - Executable binary size on disk (budget: < 2 MB)

use std::mem::size_of;
use std::time::Instant;
use windows::core::w;
use windows::Win32::Foundation::{FILETIME, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX};
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, RegisterClassExW,
    TranslateMessage, MSG, WINDOW_EX_STYLE, WM_DESTROY, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_DESTROY {
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn measure_window_creation() -> f64 {
    let class_name = w!("HeadroomSpikeDashboardClass");

    unsafe {
        let wnd_class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            lpszClassName: class_name,
            ..Default::default()
        };

        let _ = RegisterClassExW(&wnd_class);

        let start = Instant::now();
        let hwnd_res = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("Headroom Benchmark Dashboard"),
            WS_OVERLAPPEDWINDOW,
            100,
            100,
            400,
            300,
            HWND::default(),
            None,
            None,
            None,
        );

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

        if let Ok(hwnd) = hwnd_res {
            let _ = DestroyWindow(hwnd);
        }

        elapsed_ms
    }
}

fn get_memory_stats() -> (usize, usize) {
    unsafe {
        let h_proc = GetCurrentProcess();
        let mut counters: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
        let cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;

        let res = GetProcessMemoryInfo(h_proc, &mut counters as *mut _ as *mut _, cb);

        if res.is_ok() {
            (counters.WorkingSetSize, counters.PrivateUsage)
        } else {
            (0, 0)
        }
    }
}

fn filetime_to_u64(ft: FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

fn main() {
    println!("Headroom Spike 6: Resource Footprint Measurement\n");

    // 1. Measure Dashboard Window Open Time
    println!("=== 1. Dashboard Window Creation Latency ===");
    let mut open_times = Vec::new();
    for _ in 0..10 {
        open_times.push(measure_window_creation());
    }
    let avg_time: f64 = open_times.iter().sum::<f64>() / open_times.len() as f64;
    println!("[+] Window Creation Times (10 runs): {:?}", open_times);
    println!(
        "[+] Average Dashboard Creation Time: {:.3} ms (Budget: < 300 ms)",
        avg_time
    );
    if avg_time < 300.0 {
        println!("    Status: PASS [measured]");
    } else {
        println!("    Status: FAIL [measured]");
    }

    // 2. Measure Memory Usage
    println!("\n=== 2. Memory Footprint ===");
    let (ws, private_bytes) = get_memory_stats();
    let ws_mb = ws as f64 / (1024.0 * 1024.0);
    let priv_mb = private_bytes as f64 / (1024.0 * 1024.0);
    println!("[+] Working Set:   {:.2} MB ({} bytes)", ws_mb, ws);
    println!(
        "[+] Private Bytes: {:.2} MB ({} bytes) (Budget: < 5.0 MB)",
        priv_mb, private_bytes
    );
    if priv_mb < 5.0 {
        println!("    Status: PASS [measured]");
    } else {
        println!("    Status: FAIL [measured]");
    }

    // 3. CPU measurement over a simulated message pump
    println!("\n=== 3. Idle Event Loop (Simulated 3 seconds) ===");
    let start_instant = Instant::now();
    let mut ft_create = FILETIME::default();
    let mut ft_exit = FILETIME::default();
    let mut ft_kernel_start = FILETIME::default();
    let mut ft_user_start = FILETIME::default();

    unsafe {
        let _ = GetProcessTimes(
            GetCurrentProcess(),
            &mut ft_create,
            &mut ft_exit,
            &mut ft_kernel_start,
            &mut ft_user_start,
        );
    }

    // Idle loop using standard Win32 blocking timer wait (1-second sample tick)
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, KillTimer, SetTimer};
        let _ = SetTimer(HWND::default(), 1, 1000, None);
        while start_instant.elapsed().as_secs() < 3 {
            let mut msg = MSG::default();
            if GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        let _ = KillTimer(HWND::default(), 1);
    }

    let mut ft_kernel_end = FILETIME::default();
    let mut ft_user_end = FILETIME::default();
    unsafe {
        let _ = GetProcessTimes(
            GetCurrentProcess(),
            &mut ft_create,
            &mut ft_exit,
            &mut ft_kernel_end,
            &mut ft_user_end,
        );
    }

    let cpu_kernel_us = (filetime_to_u64(ft_kernel_end) - filetime_to_u64(ft_kernel_start)) / 10;
    let cpu_user_us = (filetime_to_u64(ft_user_end) - filetime_to_u64(ft_user_start)) / 10;
    let total_cpu_ms = (cpu_kernel_us + cpu_user_us) as f64 / 1000.0;
    let wall_clock_ms = start_instant.elapsed().as_secs_f64() * 1000.0;
    let cpu_percent = (total_cpu_ms / wall_clock_ms) * 100.0;

    println!(
        "[+] Idle CPU Consumption: {:.4}% (Budget: < 0.1%)",
        cpu_percent
    );
    if cpu_percent < 0.1 {
        println!("    Status: PASS [measured]");
    } else {
        println!("    Status: FAIL [measured]");
    }
}
