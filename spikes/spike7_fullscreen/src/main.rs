//! Spike 7: Fullscreen and Game Detection Probe
//!
//! Evaluates whether SHQueryUserNotificationState reliably detects fullscreen games
//! (exclusive and borderless), and tests secondary detection heuristics (monitor bounds match).

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::Shell::{
    SHQueryUserNotificationState, QUERY_USER_NOTIFICATION_STATE, QUNS_ACCEPTS_NOTIFICATIONS,
    QUNS_APP, QUNS_BUSY, QUNS_NOT_PRESENT, QUNS_PRESENTATION_MODE, QUNS_QUIET_TIME,
    QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
};

fn quns_to_string(state: QUERY_USER_NOTIFICATION_STATE) -> &'static str {
    match state {
        QUNS_NOT_PRESENT => "QUNS_NOT_PRESENT (1): User not present",
        QUNS_BUSY => "QUNS_BUSY (2): Busy / full-screen app running or presentation",
        QUNS_RUNNING_D3D_FULL_SCREEN => {
            "QUNS_RUNNING_D3D_FULL_SCREEN (3): Direct3D fullscreen game active"
        }
        QUNS_PRESENTATION_MODE => "QUNS_PRESENTATION_MODE (4): Presentation mode active",
        QUNS_ACCEPTS_NOTIFICATIONS => {
            "QUNS_ACCEPTS_NOTIFICATIONS (5): Normal desktop, accepts notifications"
        }
        QUNS_QUIET_TIME => "QUNS_QUIET_TIME (6): Quiet time",
        QUNS_APP => "QUNS_APP (7): Windows Store / UWP app in fullscreen",
        _ => "Unknown QUNS state",
    }
}

fn check_foreground_window() {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.0.is_null() {
            println!("[-] No foreground window detected");
            return;
        }

        let len = GetWindowTextLengthW(hwnd);
        let mut title_buf = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..copied as usize]);
        println!("[*] Foreground Window Title: \"{}\"", title);

        let mut win_rect = RECT::default();
        if GetWindowRect(hwnd, &mut win_rect).is_ok() {
            println!(
                "    Window Rect: [left: {}, top: {}, right: {}, bottom: {}] (width: {}, height: {})",
                win_rect.left, win_rect.top, win_rect.right, win_rect.bottom,
                win_rect.right - win_rect.left, win_rect.bottom - win_rect.top
            );

            let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
            let mut mon_info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(hmon, &mut mon_info).as_bool() {
                let mon_rect = mon_info.rcMonitor;
                println!(
                    "    Monitor Rect: [left: {}, top: {}, right: {}, bottom: {}]",
                    mon_rect.left, mon_rect.top, mon_rect.right, mon_rect.bottom
                );

                let is_fullscreen_dimensions = win_rect.left <= mon_rect.left
                    && win_rect.top <= mon_rect.top
                    && win_rect.right >= mon_rect.right
                    && win_rect.bottom >= mon_rect.bottom;

                println!(
                    "    Matches monitor dimensions (borderless fullscreen test): {}",
                    is_fullscreen_dimensions
                );
            }
        }
    }
}

fn main() {
    println!("Headroom Spike 7: Fullscreen / Game Detection\n");

    let res = unsafe { SHQueryUserNotificationState() };
    match res {
        Ok(state) => {
            println!(
                "[+] SHQueryUserNotificationState result: {}",
                quns_to_string(state)
            );
            let is_gaming_or_fullscreen = matches!(
                state,
                QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_BUSY | QUNS_PRESENTATION_MODE
            );
            println!(
                "[+] Policy Fullscreen Inhibit Recommended: {}",
                is_gaming_or_fullscreen
            );
        }
        Err(e) => {
            println!("[-] SHQueryUserNotificationState failed: {:?}", e);
        }
    }

    println!("\n=== Foreground Window Rect Heuristic ===");
    check_foreground_window();
}
