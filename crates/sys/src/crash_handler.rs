//! Local crash and panic logger.
//!
//! # Safety & Zero-Telemetry Guarantee
//! All crash forensics are written strictly locally to disk (`%LOCALAPPDATA%\Headroom\crash.log`
//! or adjacent to the executable in portable mode). Zero network requests are made.

use std::fs::OpenOptions;
use std::io::Write;
use std::panic;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the path to the local crash log file.
fn get_crash_log_path() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let portable_marker = parent.join("portable");
            if portable_marker.exists() || parent.join("headroom.json").exists() {
                return parent.join("crash.log");
            }
        }
    }

    if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        let dir = PathBuf::from(local_appdata).join("Headroom");
        let _ = std::fs::create_dir_all(&dir);
        dir.join("crash.log")
    } else {
        PathBuf::from("crash.log")
    }
}

/// Installs a local panic logger that writes unhandled panic information to `crash.log`.
pub fn install_crash_handler(component_name: &'static str) {
    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |panic_info| {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let pid = std::process::id();
        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "Unknown panic payload"
        };

        let location = if let Some(loc) = panic_info.location() {
            format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
        } else {
            "Unknown location".to_string()
        };

        let log_entry = format!(
            "[{}] COMPONENT: {} | PID: {} | LOCATION: {} | PAYLOAD: {}\n",
            timestamp, component_name, pid, location, payload
        );

        let log_path = get_crash_log_path();
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
            let _ = file.write_all(log_entry.as_bytes());
        }

        eprintln!(
            "\n[Headroom Fatal Error] Process crashed. Diagnostic details written to: {:?}\n",
            log_path
        );

        default_hook(panic_info);
    }));
}
