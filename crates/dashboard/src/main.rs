//! Headroom On-Demand Diagnostic Dashboard.
//!
//! # Responsibilities
//! - Instant startup (< 300 ms latency).
//! - Native Win32 GDI double-buffered rendering with Per-Monitor DPI v2 awareness.
//! - Dark theme support via DwmSetWindowAttribute.
//! - Live breakdown of In Use, Modified, Standby (Reserve/Normal/Core), Free/Zero, Commit, and Hard Faults.
//! - Diagnostic banner with plain-language explanations.
//! - Top memory consumer processes with private and working set usage.
//! - Exits completely on window close, releasing 100% of its memory.

use headroom_core::{classify, Config, MemoryHistory, MemorySample, StateDiagnosis, SystemState};
use headroom_engine::CleaningEngine;
use headroom_ipc::ActionKind;
use headroom_sys::{install_crash_handler, is_process_elevated, SystemSampler};
use std::mem::size_of;
use std::sync::{Arc, Mutex};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreatePen, CreateSolidBrush,
    DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, FrameRect, InvalidateRect, LineTo,
    MoveToEx, SelectObject, SetBkMode, SetTextColor, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_WORDBREAK,
    HDC, PAINTSTRUCT, PS_SOLID, SRCCOPY, TRANSPARENT,
};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW,
    GetWindowLongPtrW, KillTimer, PostQuitMessage, RegisterClassExW, SetTimer, SetWindowLongPtrW,
    TranslateMessage, GWLP_USERDATA, HMENU, MINMAXINFO, MSG, WINDOW_EX_STYLE, WM_COMMAND,
    WM_DESTROY, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_PAINT, WM_SIZE, WM_TIMER, WNDCLASSEXW,
    WS_CHILD, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

const TIMER_REFRESH_ID: usize = 1;
const ID_BTN_TRIM: usize = 2001;
const ID_BTN_PURGE_LOW: usize = 2002;
const ID_BTN_TOGGLE_EXCLUDE: usize = 2003;
const ID_BTN_CHECK_UPDATE: usize = 2004;

struct DashboardState {
    sampler: SystemSampler,
    config: Config,
    sample: Option<MemorySample>,
    diagnosis: Option<StateDiagnosis>,
    history: MemoryHistory,
    engine: Arc<CleaningEngine>,
    status_message: Arc<Mutex<Option<String>>>,
    is_elevated: bool,
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut DashboardState;

    match msg {
        WM_TIMER => {
            if wparam.0 == TIMER_REFRESH_ID && !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if let Ok(sample) = state.sampler.sample(6) {
                    let diag = classify(&sample, &state.config);
                    state.history.record(&sample);
                    state.sample = Some(sample);
                    state.diagnosis = Some(diag);
                    let _ = InvalidateRect(hwnd, None, false);
                }
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let cmd_id = wparam.0 & 0xFFFF;
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                match cmd_id {
                    ID_BTN_TRIM => {
                        let engine = Arc::clone(&state.engine);
                        let status = Arc::clone(&state.status_message);
                        let cfg = state.config.clone();
                        if let Some(s) = &state.sample {
                            let pids: Vec<u32> = s
                                .top_consumers
                                .iter()
                                .filter(|p| !cfg.is_excluded(&p.name))
                                .map(|p| p.pid)
                                .collect();
                            std::thread::spawn(move || {
                                let mut count = 0;
                                for pid in pids {
                                    if engine
                                        .execute(
                                            ActionKind::TrimProcess,
                                            Some(pid),
                                            "dashboard_ui",
                                            &cfg,
                                        )
                                        .is_ok()
                                    {
                                        count += 1;
                                    }
                                }
                                *status.lock().unwrap() = Some(format!(
                                    "[+] Trimmed {} background processes (Receipt logged)",
                                    count
                                ));
                            });
                        }
                    }
                    ID_BTN_PURGE_LOW => {
                        let engine = Arc::clone(&state.engine);
                        let status = Arc::clone(&state.status_message);
                        let cfg = state.config.clone();
                        std::thread::spawn(move || {
                            match engine.execute(
                                ActionKind::PurgeLowPriorityStandby,
                                None,
                                "dashboard_ui",
                                &cfg,
                            ) {
                                Ok(r) => {
                                    *status.lock().unwrap() = Some(format!(
                                        "[+] Low-priority standby purged ({} ms, Receipt logged)",
                                        r.duration_ms
                                    ));
                                }
                                Err(e) => {
                                    *status.lock().unwrap() =
                                        Some(format!("[-] Action skipped: {}", e));
                                }
                            }
                        });
                    }
                    ID_BTN_TOGGLE_EXCLUDE => {
                        let status = Arc::clone(&state.status_message);
                        if let Some(s) = &state.sample {
                            if let Some(top_p) = s.top_consumers.first() {
                                let proc_name = top_p.name.clone();
                                let is_now_excluded = state.config.toggle_exclusion(&proc_name);
                                let save_res = state.config.save_to_file();
                                let action_str = if is_now_excluded {
                                    "Excluded"
                                } else {
                                    "Un-excluded"
                                };
                                match save_res {
                                    Ok(_) => {
                                        *status.lock().unwrap() = Some(format!(
                                            "[+] {} '{}' from trimming (saved to config)",
                                            action_str, proc_name
                                        ));
                                    }
                                    Err(e) => {
                                        *status.lock().unwrap() = Some(format!(
                                            "[!] {} '{}' (config save warning: {})",
                                            action_str, proc_name, e
                                        ));
                                    }
                                }
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                        }
                    }
                    ID_BTN_CHECK_UPDATE => {
                        let status = Arc::clone(&state.status_message);
                        std::thread::spawn(move || {
                            *status.lock().unwrap() =
                                Some("[*] Checking GitHub for latest version...".to_string());
                            let output = std::process::Command::new("powershell")
                                .args([
                                    "-NoProfile",
                                    "-Command",
                                    "try { (Invoke-RestMethod -Uri 'https://api.github.com/repos/redwan2003-bot/HeadRoom/releases/latest' -TimeoutSec 3).tag_name } catch { 'OFFLINE' }",
                                ])
                                .output();
                            if let Ok(out) = output {
                                let tag = String::from_utf8_lossy(&out.stdout).trim().to_string();
                                if tag == "OFFLINE" || tag.is_empty() {
                                    *status.lock().unwrap() = Some(
                                        "[-] Update check: Network offline or timeout.".to_string(),
                                    );
                                } else if tag == "v1.1.0" {
                                    *status.lock().unwrap() =
                                        Some(format!("[+] Headroom is up to date ({})", tag));
                                } else {
                                    *status.lock().unwrap() = Some(format!(
                                        "[★] New release available: {} (Visit GitHub Releases)",
                                        tag
                                    ));
                                }
                            } else {
                                *status.lock().unwrap() =
                                    Some("[-] Unable to execute update check.".to_string());
                            }
                        });
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_SIZE => {
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        WM_GETMINMAXINFO => {
            if lparam.0 != 0 {
                let mmi = &mut *(lparam.0 as *mut MINMAXINFO);
                mmi.ptMinTrackSize.x = 680;
                mmi.ptMinTrackSize.y = 600;
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1), // Suppress erase background for flicker-free double buffering
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            paint_dashboard(hwnd, hdc, state_ptr);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            let _ = KillTimer(hwnd, TIMER_REFRESH_ID);
            if !state_ptr.is_null() {
                let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let _ = Box::from_raw(state_ptr);
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn paint_dashboard(hwnd: HWND, hdc: HDC, state_ptr: *const DashboardState) {
    unsafe {
        let mut client_rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut client_rect);
        let width = client_rect.right - client_rect.left;
        let height = client_rect.bottom - client_rect.top;

        // Double buffer DC
        let mem_dc = CreateCompatibleDC(hdc);
        let mem_bmp = CreateCompatibleBitmap(hdc, width, height);
        let old_bmp = SelectObject(mem_dc, mem_bmp);

        // Dark Theme Colors
        let bg_color = COLORREF(0x001E1E1E); // #1E1E1E
        let text_main = COLORREF(0x00E0E0E0);
        let text_muted = COLORREF(0x009E9E9E);

        let bg_brush = CreateSolidBrush(bg_color);
        let _ = FillRect(mem_dc, &client_rect, bg_brush);
        let _ = DeleteObject(bg_brush);

        let _ = SetBkMode(mem_dc, TRANSPARENT);

        if !state_ptr.is_null() {
            let dash = &*state_ptr;
            let mut y = 20;

            // 1. Diagnostic Banner
            if let Some(diag) = &dash.diagnosis {
                let banner_color = match diag.state {
                    SystemState::Healthy => COLORREF(0x00388E3C),   // Green
                    SystemState::CacheFull => COLORREF(0x007B1FA2), // Purple (Cache full is normal)
                    SystemState::UnderPressure => COLORREF(0x00F57C00), // Orange
                    SystemState::CommitRisk => COLORREF(0x00D32F2F), // Red
                };

                let banner_brush = CreateSolidBrush(banner_color);
                let banner_rect = RECT {
                    left: 20,
                    top: y,
                    right: width - 20,
                    bottom: y + 70,
                };
                let _ = FillRect(mem_dc, &banner_rect, banner_brush);
                let _ = DeleteObject(banner_brush);

                let _ = SetTextColor(mem_dc, COLORREF(0x00FFFFFF));
                let mut headline_wide: Vec<u16> = diag.headline.encode_utf16().collect();
                let mut text_rect = RECT {
                    left: 30,
                    top: y + 8,
                    right: width - 30,
                    bottom: y + 30,
                };
                let _ = DrawTextW(
                    mem_dc,
                    &mut headline_wide,
                    &mut text_rect,
                    DT_LEFT | DT_NOPREFIX,
                );

                let priv_str = if dash.is_elevated {
                    "[Elevated / Service Active]"
                } else {
                    "[Standard / Portable]"
                };
                let mut priv_wide: Vec<u16> = priv_str.encode_utf16().collect();
                let mut priv_rect = RECT {
                    left: width - 260,
                    top: y + 8,
                    right: width - 30,
                    bottom: y + 30,
                };
                let _ = DrawTextW(
                    mem_dc,
                    &mut priv_wide,
                    &mut priv_rect,
                    DT_RIGHT | DT_NOPREFIX,
                );

                let mut expl_wide: Vec<u16> = diag.explanation.encode_utf16().collect();
                let mut expl_rect = RECT {
                    left: 30,
                    top: y + 32,
                    right: width - 30,
                    bottom: y + 65,
                };
                let _ = DrawTextW(
                    mem_dc,
                    &mut expl_wide,
                    &mut expl_rect,
                    DT_LEFT | DT_WORDBREAK | DT_NOPREFIX,
                );

                y += 85;
            }

            // 2. Memory Breakdown Grid
            if let Some(s) = &dash.sample {
                let total_mb = s.physical_total_bytes / (1024 * 1024);
                let in_use_mb = s.in_use_bytes / (1024 * 1024);
                let avail_mb = s.available_bytes / (1024 * 1024);
                let standby_mb = s.standby.total() / (1024 * 1024);
                let mod_mb = s.modified_bytes / (1024 * 1024);
                let free_mb = s.free_zero_bytes / (1024 * 1024);

                let commit_mb = s.commit.total_bytes / (1024 * 1024);
                let commit_lim_mb = s.commit.limit_bytes / (1024 * 1024);

                let _ = SetTextColor(mem_dc, text_main);

                let lines = [
                    format!("Physical RAM: {} MB Total", total_mb),
                    format!(
                        "In Use:       {} MB ({:.1}%)",
                        in_use_mb,
                        (in_use_mb as f64 / total_mb as f64) * 100.0
                    ),
                    format!(
                        "Available:    {} MB ({:.1}%)",
                        avail_mb,
                        (avail_mb as f64 / total_mb as f64) * 100.0
                    ),
                    format!(
                        "Standby Cache: {} MB (Reserve: {} MB, Normal: {} MB, Core: {} MB)",
                        standby_mb,
                        s.standby.reserve_bytes / (1024 * 1024),
                        s.standby.normal_bytes / (1024 * 1024),
                        s.standby.core_bytes / (1024 * 1024)
                    ),
                    format!("Modified List: {} MB (pending flush to disk)", mod_mb),
                    format!("Free & Zero:  {} MB", free_mb),
                    format!(
                        "Commit Total: {} MB / {} MB Limit ({}%)",
                        commit_mb,
                        commit_lim_mb,
                        s.commit.percent()
                    ),
                    format!(
                        "Hard Page Faults: {:.1} pages read from disk / sec",
                        s.hard_faults_per_sec
                    ),
                ];

                for line in &lines {
                    let mut wide: Vec<u16> = line.encode_utf16().collect();
                    let mut r = RECT {
                        left: 25,
                        top: y,
                        right: width - 25,
                        bottom: y + 20,
                    };
                    let _ = DrawTextW(mem_dc, &mut wide, &mut r, DT_LEFT | DT_NOPREFIX);
                    y += 22;
                }

                // 3. Mini GDI Sparkline (10-minute rolling RAM In-Use trend)
                y += 6;
                let pts = dash.history.get_recent_10_minutes();
                let (cur_pct, peak_pct) = if !pts.is_empty() && total_mb > 0 {
                    let mut max_in_use = 0u64;
                    for pt in &pts {
                        let u = total_mb.saturating_sub(pt.available_mb as u64);
                        if u > max_in_use {
                            max_in_use = u;
                        }
                    }
                    let last = pts.last().unwrap();
                    let cur = total_mb.saturating_sub(last.available_mb as u64);
                    (
                        (cur as f64 / total_mb as f64) * 100.0,
                        (max_in_use as f64 / total_mb as f64) * 100.0,
                    )
                } else {
                    (
                        (in_use_mb as f64 / total_mb.max(1) as f64) * 100.0,
                        (in_use_mb as f64 / total_mb.max(1) as f64) * 100.0,
                    )
                };

                let chart_title = format!(
                    "RAM In-Use Trend (10-min rolling): Current: {:.1}% | Peak: {:.1}%",
                    cur_pct, peak_pct
                );
                let mut wide_chart_title: Vec<u16> = chart_title.encode_utf16().collect();
                let mut r_chart_title = RECT {
                    left: 25,
                    top: y,
                    right: width - 25,
                    bottom: y + 18,
                };
                let _ = DrawTextW(
                    mem_dc,
                    &mut wide_chart_title,
                    &mut r_chart_title,
                    DT_LEFT | DT_NOPREFIX,
                );
                y += 20;

                let spark_rect = RECT {
                    left: 25,
                    top: y,
                    right: width - 25,
                    bottom: y + 46,
                };
                paint_sparkline(mem_dc, &spark_rect, &dash.history, total_mb);
                y += 54;

                // 4. Top Process Consumers
                let header = "Top Process Memory Consumers:".to_string();
                let mut wide_hdr: Vec<u16> = header.encode_utf16().collect();
                let mut r_hdr = RECT {
                    left: 25,
                    top: y,
                    right: width - 25,
                    bottom: y + 18,
                };
                let _ = DrawTextW(mem_dc, &mut wide_hdr, &mut r_hdr, DT_LEFT | DT_NOPREFIX);
                y += 20;

                let _ = SetTextColor(mem_dc, text_muted);
                for p in s.top_consumers.iter().take(6) {
                    let is_ex = dash.config.is_excluded(&p.name);
                    let ex_tag = if is_ex { " [EXCLUDED]" } else { "" };
                    let proc_line = format!(
                        "  [{:>5}] {:<22} Private: {:>5} MB  |  WS: {:>5} MB{}",
                        p.pid,
                        p.name,
                        p.private_bytes / (1024 * 1024),
                        p.working_set_bytes / (1024 * 1024),
                        ex_tag
                    );
                    let mut wide_proc: Vec<u16> = proc_line.encode_utf16().collect();
                    let mut r_p = RECT {
                        left: 25,
                        top: y,
                        right: width - 25,
                        bottom: y + 18,
                    };
                    let _ = DrawTextW(mem_dc, &mut wide_proc, &mut r_p, DT_LEFT | DT_NOPREFIX);
                    y += 18;
                }

                // 5. Status Message Feedback
                if let Ok(guard) = dash.status_message.lock() {
                    if let Some(msg) = guard.as_ref() {
                        let _ = SetTextColor(mem_dc, COLORREF(0x004CAF50));
                        let mut wide_msg: Vec<u16> = msg.encode_utf16().collect();
                        let mut r_msg = RECT {
                            left: 25,
                            top: y + 4,
                            right: width - 25,
                            bottom: y + 24,
                        };
                        let _ = DrawTextW(mem_dc, &mut wide_msg, &mut r_msg, DT_LEFT | DT_NOPREFIX);
                    }
                }
            }
        }

        // Blit back buffer to screen DC
        let _ = BitBlt(hdc, 0, 0, width, height, mem_dc, 0, 0, SRCCOPY);

        // Cleanup GDI objects
        let _ = SelectObject(mem_dc, old_bmp);
        let _ = DeleteObject(mem_bmp);
        let _ = DeleteDC(mem_dc);
    }
}

fn paint_sparkline(mem_dc: HDC, rect: &RECT, history: &MemoryHistory, total_ram_mb: u64) {
    unsafe {
        // Dark panel background
        let bg_brush = CreateSolidBrush(COLORREF(0x00141414));
        let _ = FillRect(mem_dc, rect, bg_brush);
        let _ = DeleteObject(bg_brush);

        // Border
        let border_brush = CreateSolidBrush(COLORREF(0x00333333));
        let _ = FrameRect(mem_dc, rect, border_brush);
        let _ = DeleteObject(border_brush);

        if total_ram_mb == 0 {
            return;
        }

        let box_w = rect.right - rect.left - 4;
        let box_h = rect.bottom - rect.top - 4;
        if box_w <= 10 || box_h <= 10 {
            return;
        }

        // Draw 50% dotted grid line
        let grid_pen = CreatePen(PS_SOLID, 1, COLORREF(0x002A2A2A));
        if !grid_pen.is_invalid() {
            let old_pen = SelectObject(mem_dc, grid_pen);
            let mid_y = rect.top + 2 + box_h / 2;
            let _ = MoveToEx(mem_dc, rect.left + 2, mid_y, None);
            let _ = LineTo(mem_dc, rect.right - 2, mid_y);
            let _ = SelectObject(mem_dc, old_pen);
            let _ = DeleteObject(grid_pen);
        }

        let pts = history.get_recent_10_minutes();
        if pts.is_empty() {
            return;
        }

        // Draw Cyan Sparkline (0x00D4BC00 is COLORREF for #00BCD4 Cyan in BGR)
        let spark_pen = CreatePen(PS_SOLID, 2, COLORREF(0x00D4BC00));
        if !spark_pen.is_invalid() {
            let old_pen = SelectObject(mem_dc, spark_pen);
            let n_pts = pts.len();

            for (i, pt) in pts.iter().enumerate() {
                let in_use_mb = total_ram_mb.saturating_sub(pt.available_mb as u64);
                let pct = (in_use_mb as f64 / total_ram_mb as f64).clamp(0.0, 1.0);

                let x = if n_pts <= 1 {
                    rect.left + 2 + box_w
                } else {
                    rect.left + 2 + (i as i32 * box_w) / ((n_pts - 1) as i32)
                };
                let y = rect.top + 2 + ((1.0 - pct) * (box_h as f64)) as i32;

                if i == 0 {
                    let _ = MoveToEx(mem_dc, x, y, None);
                } else {
                    let _ = LineTo(mem_dc, x, y);
                }
            }

            let _ = SelectObject(mem_dc, old_pen);
            let _ = DeleteObject(spark_pen);
        }
    }
}

fn main() {
    let start_instant = std::time::Instant::now();
    let is_bench = std::env::args().any(|arg| arg == "--bench");

    // 0. Install Local Zero-Telemetry Crash Handler
    install_crash_handler("headroom-dashboard");

    // 1. Per-Monitor DPI v2
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    // 2. Initialize Telemetry & State (loading persisted user config)
    let (config, _) = Config::load_or_default();
    let is_elevated = is_process_elevated();
    let mut sampler = match SystemSampler::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize dashboard telemetry: {}", e);
            return;
        }
    };

    // Immediate initial sample without blocking process enumeration for instant (< 50ms) window display
    let initial_sample = sampler.sample(0).ok();
    let initial_diag = initial_sample.as_ref().map(|s| classify(s, &config));

    let mut history = MemoryHistory::default();
    if let Some(s) = &initial_sample {
        history.record(s);
    }

    let dashboard_state = Box::new(DashboardState {
        sampler,
        config,
        sample: initial_sample,
        diagnosis: initial_diag,
        history,
        engine: Arc::new(CleaningEngine::default()),
        status_message: Arc::new(Mutex::new(None)),
        is_elevated,
    });

    // 3. Register Window Class
    let class_name = w!("HeadroomDashboardClass");
    unsafe {
        let wnd_class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            lpszClassName: class_name,
            ..Default::default()
        };
        let _ = RegisterClassExW(&wnd_class);

        // 4. Create Window
        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("Headroom Diagnostic Dashboard"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            120,
            120,
            680,
            600,
            HWND::default(),
            None,
            None,
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("Failed to create dashboard window: {:?}", e);
                return;
            }
        };

        let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(dashboard_state) as isize);

        // 5. Create Interactive Action Buttons
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BUTTON"),
            w!("Trim Background"),
            WS_CHILD | WS_VISIBLE,
            25,
            515,
            155,
            32,
            hwnd,
            HMENU(ID_BTN_TRIM as *mut _),
            HINSTANCE::default(),
            None,
        );

        let _ = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BUTTON"),
            w!("Purge Standby"),
            WS_CHILD | WS_VISIBLE,
            185,
            515,
            155,
            32,
            hwnd,
            HMENU(ID_BTN_PURGE_LOW as *mut _),
            HINSTANCE::default(),
            None,
        );

        let _ = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BUTTON"),
            w!("Toggle Exclude Top"),
            WS_CHILD | WS_VISIBLE,
            345,
            515,
            170,
            32,
            hwnd,
            HMENU(ID_BTN_TOGGLE_EXCLUDE as *mut _),
            HINSTANCE::default(),
            None,
        );

        let _ = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BUTTON"),
            w!("Check Updates"),
            WS_CHILD | WS_VISIBLE,
            520,
            515,
            135,
            32,
            hwnd,
            HMENU(ID_BTN_CHECK_UPDATE as *mut _),
            HINSTANCE::default(),
            None,
        );

        // 6. Enable Immersive Dark Mode
        let dark_mode: i32 = 1;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark_mode as *const _ as *const std::ffi::c_void,
            size_of::<i32>() as u32,
        );

        // 7. Refresh Timer (1 Hz)
        let _ = SetTimer(hwnd, TIMER_REFRESH_ID, 1000, None);

        let elapsed_ms = start_instant.elapsed().as_secs_f64() * 1000.0;
        if is_bench {
            println!(
                "Dashboard Window Creation Latency: {:.3} ms (Budget: < 300 ms)",
                elapsed_ms
            );
            let _ = DestroyWindow(hwnd);
            return;
        }

        // 7. Message Loop
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
