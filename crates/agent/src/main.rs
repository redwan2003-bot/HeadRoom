//! Headroom Persistent Background Tray Agent.
//!
//! # Responsibilities
//! - Runs unelevated with minimal footprint (<5 MB private memory, <0.1% idle CPU).
//! - Enforces single-instance via named mutex.
//! - Preallocates telemetry structures to avoid allocations under memory pressure.
//! - Monitors memory metrics and updates system tray tooltip and icon.
//! - Handles `TaskbarCreated` message to restore icon when Explorer restarts.
//! - Spawns on-demand dashboard window on interaction.

use headroom_core::{classify, Config, HysteresisFilter, MemorySample, SystemState};
use headroom_sys::SystemSampler;
use std::mem::size_of;
use std::process::Command;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HINSTANCE, HWND, LPARAM, LRESULT, POINT,
    WPARAM,
};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, GetWindowLongPtrW, LoadIconW, PostQuitMessage,
    RegisterClassExW, RegisterWindowMessageW, SetForegroundWindow, SetTimer, SetWindowLongPtrW,
    TrackPopupMenu, TranslateMessage, GWLP_USERDATA, HICON, IDI_APPLICATION, MF_SEPARATOR,
    MF_STRING, MSG, TPM_BOTTOMALIGN, TPM_LEFTALIGN, WINDOW_EX_STYLE, WM_COMMAND, WM_DESTROY,
    WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP, WM_TIMER, WM_USER, WNDCLASSEXW, WS_OVERLAPPED,
};

use std::alloc::System;

#[global_allocator]
static GLOBAL: System = System;

const WM_TRAYICON: u32 = WM_USER + 101;
const ID_TRAY_OPEN_DASHBOARD: usize = 1001;
const ID_TRAY_STATUS_INFO: usize = 1002;
const ID_TRAY_EXIT: usize = 1003;
const TIMER_SAMPLE_ID: usize = 1;

static mut TASKBAR_CREATED_MSG: u32 = 0;

struct AgentApp {
    sampler: SystemSampler,
    config: Config,
    filter: HysteresisFilter,
    latest_sample: Option<MemorySample>,
    latest_state: SystemState,
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let app_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AgentApp;

    // Intercept TaskbarCreated message to re-register tray icon
    if msg == TASKBAR_CREATED_MSG && TASKBAR_CREATED_MSG != 0 {
        if !app_ptr.is_null() {
            let app = &*app_ptr;
            update_tray_icon(hwnd, app.latest_state, app.latest_sample.as_ref(), true);
        }
        return LRESULT(0);
    }

    match msg {
        WM_TIMER => {
            if wparam.0 == TIMER_SAMPLE_ID && !app_ptr.is_null() {
                let app = &mut *app_ptr;
                if let Ok(sample) = app.sampler.sample(0) {
                    let diagnosis = classify(&sample, &app.config);
                    let debounced_state = app.filter.update(&diagnosis);
                    app.latest_sample = Some(sample);
                    app.latest_state = debounced_state;
                    update_tray_icon(hwnd, debounced_state, app.latest_sample.as_ref(), false);
                }
            }
            LRESULT(0)
        }
        WM_TRAYICON => {
            let event = (lparam.0 & 0xFFFF) as u32;
            match event {
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                    spawn_dashboard();
                }
                WM_RBUTTONUP => {
                    show_tray_menu(hwnd, app_ptr);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let cmd_id = wparam.0 & 0xFFFF;
            match cmd_id {
                ID_TRAY_OPEN_DASHBOARD => spawn_dashboard(),
                ID_TRAY_EXIT => {
                    let _ = DestroyWindow(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            remove_tray_icon(hwnd);
            if !app_ptr.is_null() {
                let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let _ = Box::from_raw(app_ptr);
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn spawn_dashboard() {
    let _ = Command::new("headroom-dashboard.exe").spawn();
}

fn show_tray_menu(hwnd: HWND, app_ptr: *const AgentApp) {
    unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);

        let h_menu = CreatePopupMenu();
        if let Ok(menu) = h_menu {
            let status_label = if !app_ptr.is_null() {
                format!("State: {}", (*app_ptr).latest_state.label())
            } else {
                "State: Initializing".to_string()
            };
            let wide_status: Vec<u16> = status_label
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let _ = AppendMenuW(
                menu,
                MF_STRING,
                ID_TRAY_OPEN_DASHBOARD,
                w!("Open Dashboard"),
            );
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                ID_TRAY_STATUS_INFO,
                PCWSTR::from_raw(wide_status.as_ptr()),
            );
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
            let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_EXIT, w!("Exit Headroom"));

            let _ = SetForegroundWindow(hwnd);
            let _ = TrackPopupMenu(
                menu,
                TPM_BOTTOMALIGN | TPM_LEFTALIGN,
                pt.x,
                pt.y,
                0,
                hwnd,
                None,
            );
            let _ = DestroyMenu(menu);
        }
    }
}

fn update_tray_icon(
    hwnd: HWND,
    state: SystemState,
    sample_opt: Option<&MemorySample>,
    is_add: bool,
) {
    let tooltip = if let Some(sample) = sample_opt {
        let avail_mb = sample.available_bytes / (1024 * 1024);
        let in_use_pct =
            ((sample.in_use_bytes as f64 / sample.physical_total_bytes as f64) * 100.0) as u8;
        format!(
            "Headroom: {}\nIn Use: {}% | Avail: {} MB\nCommit: {}%",
            state.label(),
            in_use_pct,
            avail_mb,
            sample.commit.percent()
        )
    } else {
        "Headroom: Initializing...".to_string()
    };

    let mut tip_wide = [0u16; 128];
    for (i, c) in tooltip.encode_utf16().take(127).enumerate() {
        tip_wide[i] = c;
    }

    unsafe {
        let h_icon = LoadIconW(HINSTANCE::default(), IDI_APPLICATION).unwrap_or(HICON::default());

        let nid = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: h_icon,
            szTip: tip_wide,
            ..Default::default()
        };

        if is_add {
            let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        } else {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }
}

fn remove_tray_icon(hwnd: HWND) {
    unsafe {
        let nid = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            ..Default::default()
        };
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

fn main() {
    // 1. Enforce Per-Monitor DPI v2
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    // 2. Single-Instance Check
    let mutex_name = w!("Headroom_Agent_SingleInstance");
    let h_mutex = unsafe { CreateMutexW(None, true, mutex_name) };
    if h_mutex.is_err() || unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        eprintln!("Headroom Agent is already running.");
        return;
    }

    // 3. Preallocate State & Sampler
    let config = Config::default();
    let filter = HysteresisFilter::new(config.hysteresis_samples);
    let sampler = match SystemSampler::new_lightweight() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize telemetry: {}", e);
            return;
        }
    };

    // Trim initial startup footprint
    unsafe {
        use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
        use windows::Win32::System::Threading::GetCurrentProcess;
        let _ = EmptyWorkingSet(GetCurrentProcess());
    }

    // 4. Register Window Class & Window
    let class_name = w!("HeadroomAgentWindowClass");
    unsafe {
        let wnd_class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            lpszClassName: class_name,
            ..Default::default()
        };
        let _ = RegisterClassExW(&wnd_class);

        // Intercept TaskbarCreated message
        TASKBAR_CREATED_MSG = RegisterWindowMessageW(w!("TaskbarCreated"));

        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("Headroom Agent"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            HWND::default(),
            None,
            None,
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("Failed to create agent window: {:?}", e);
                return;
            }
        };

        let app_box = Box::new(AgentApp {
            sampler,
            config,
            filter,
            latest_sample: None,
            latest_state: SystemState::Healthy,
        });
        let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(app_box) as isize);

        // Add initial tray icon
        update_tray_icon(hwnd, SystemState::Healthy, None, true);

        // 1-second sample timer
        let _ = SetTimer(hwnd, TIMER_SAMPLE_ID, 1000, None);

        // 5. Message Loop (Blocking wait, 0% idle CPU)
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Cleanup mutex
        if let Ok(mutex) = h_mutex {
            let _ = CloseHandle(mutex);
        }
    }
}
