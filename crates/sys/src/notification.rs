//! Fullscreen and user notification state detection.

use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::Shell::{
    SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect};

/// Checks whether an exclusive or borderless fullscreen game or presentation is active.
pub fn is_fullscreen_or_gaming_active() -> bool {
    // 1. Check primary Windows Shell notification state
    // SAFETY: Calling documented SHQueryUserNotificationState API.
    let quns_res = unsafe { SHQueryUserNotificationState() };
    if let Ok(state) = quns_res {
        if matches!(
            state,
            QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_BUSY | QUNS_PRESENTATION_MODE
        ) {
            return true;
        }
    }

    // 2. Secondary check: check if foreground window occupies the entire primary monitor (borderless fullscreen)
    // SAFETY: Calling GetForegroundWindow and GetWindowRect.
    unsafe {
        let hwnd = GetForegroundWindow();
        if !hwnd.0.is_null() {
            let mut win_rect = RECT::default();
            if GetWindowRect(hwnd, &mut win_rect).is_ok() {
                let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
                let mut mon_info = MONITORINFO {
                    cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                    ..Default::default()
                };
                if GetMonitorInfoW(hmon, &mut mon_info).as_bool() {
                    let mon_rect = mon_info.rcMonitor;
                    let is_borderless = win_rect.left <= mon_rect.left
                        && win_rect.top <= mon_rect.top
                        && win_rect.right >= mon_rect.right
                        && win_rect.bottom >= mon_rect.bottom;
                    if is_borderless {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Returns the process ID of the active foreground window, if one exists.
pub fn get_foreground_process_id() -> Option<u32> {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    // SAFETY: GetForegroundWindow returns the top-level active HWND or null.
    // GetWindowThreadProcessId queries the owning process ID safely into a stack variable.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid = 0u32;
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid > 0 {
            Some(pid)
        } else {
            None
        }
    }
}
