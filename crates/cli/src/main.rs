//! Headroom Non-Interactive Command-Line Interface.
//!
//! # Responsibilities
//! - Scriptable memory diagnosis and cleaning.
//! - Silent execution with zero UI popups.
//! - `--json` flag for machine-readable JSON output.
//! - Stable exit codes:
//!   - 0: Success
//!   - 1: Aborted by policy / commit risk
//!   - 2: Invalid command-line arguments
//!   - 3: Broker or IPC unavailable
//!   - 4: Execution timeout

use headroom_core::{classify, Config};
use headroom_engine::{CleaningEngine, EngineError};
use headroom_ipc::ActionKind;
use headroom_sys::SystemSampler;
use std::process::exit;

fn print_usage() {
    eprintln!(
        r#"Headroom Memory Utility CLI

USAGE:
    headroom status [--json]
    headroom clean <ACTION> [--json]

ACTIONS:
    standby-low     Purge low-priority standby list (gamer stutter profile)
    standby-all     Purge entire standby list (requires broker)
    modified        Flush modified list pages to disk (requires broker)
    working-sets    Empty all working sets (requires broker)

FLAGS:
    --json          Output results in JSON format
    --help          Show this help message
"#
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        exit(2);
    }

    let is_json = args.iter().any(|a| a == "--json");
    let cmd = &args[0];

    match cmd.as_str() {
        "status" => cmd_status(is_json),
        "clean" => {
            if args.len() < 2 {
                eprintln!("Error: Missing clean action.");
                print_usage();
                exit(2);
            }
            let action_arg = &args[1];
            let action = match action_arg.as_str() {
                "standby-low" => ActionKind::PurgeLowPriorityStandby,
                "standby-all" => ActionKind::PurgeStandby,
                "modified" => ActionKind::FlushModifiedList,
                "working-sets" => ActionKind::EmptyWorkingSets,
                _ => {
                    eprintln!("Error: Unknown clean action '{}'.", action_arg);
                    print_usage();
                    exit(2);
                }
            };
            cmd_clean(action, is_json);
        }
        _ => {
            eprintln!("Error: Unknown command '{}'.", cmd);
            print_usage();
            exit(2);
        }
    }
}

fn cmd_status(json: bool) {
    let mut sampler = match SystemSampler::new() {
        Ok(s) => s,
        Err(e) => {
            if json {
                println!(r#"{{"error": "Failed to initialize telemetry: {}"}}"#, e);
            } else {
                eprintln!("[-] Failed to initialize telemetry: {}", e);
            }
            exit(3);
        }
    };

    let sample = match sampler.sample(5) {
        Ok(s) => s,
        Err(e) => {
            if json {
                println!(r#"{{"error": "Failed to sample memory: {}"}}"#, e);
            } else {
                eprintln!("[-] Failed to sample memory: {}", e);
            }
            exit(3);
        }
    };

    let cfg = Config::default();
    let diag = classify(&sample, &cfg);

    if json {
        let output = serde_json::json!({
            "timestamp_utc": sample.timestamp_utc,
            "state": diag.state.label(),
            "headline": diag.headline,
            "explanation": diag.explanation,
            "physical_total_mb": sample.physical_total_bytes / (1024 * 1024),
            "available_mb": sample.available_bytes / (1024 * 1024),
            "in_use_mb": sample.in_use_bytes / (1024 * 1024),
            "standby_mb": sample.standby.total() / (1024 * 1024),
            "modified_mb": sample.modified_bytes / (1024 * 1024),
            "free_zero_mb": sample.free_zero_bytes / (1024 * 1024),
            "commit_total_mb": sample.commit.total_bytes / (1024 * 1024),
            "commit_limit_mb": sample.commit.limit_bytes / (1024 * 1024),
            "commit_percent": sample.commit.percent(),
            "hard_faults_per_sec": sample.hard_faults_per_sec,
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!("Headroom Memory Status");
        println!(
            "State:            {} - {}",
            diag.state.label(),
            diag.headline
        );
        println!(
            "Physical RAM:     {} MB Total",
            sample.physical_total_bytes / (1024 * 1024)
        );
        println!(
            "In Use:           {} MB ({:.1}%)",
            sample.in_use_bytes / (1024 * 1024),
            (sample.in_use_bytes as f64 / sample.physical_total_bytes as f64) * 100.0
        );
        println!(
            "Available:        {} MB ({:.1}%)",
            sample.available_bytes / (1024 * 1024),
            (sample.available_bytes as f64 / sample.physical_total_bytes as f64) * 100.0
        );
        println!(
            "Standby Cache:    {} MB",
            sample.standby.total() / (1024 * 1024)
        );
        println!(
            "Modified List:    {} MB",
            sample.modified_bytes / (1024 * 1024)
        );
        println!(
            "Free & Zero:      {} MB",
            sample.free_zero_bytes / (1024 * 1024)
        );
        println!(
            "Commit Charge:    {} MB / {} MB Limit ({}%)",
            sample.commit.total_bytes / (1024 * 1024),
            sample.commit.limit_bytes / (1024 * 1024),
            sample.commit.percent()
        );
        println!(
            "Hard Faults:      {:.1} pages/sec",
            sample.hard_faults_per_sec
        );
    }

    exit(0);
}

fn cmd_clean(action: ActionKind, json: bool) {
    let cfg = Config::default();
    let engine = CleaningEngine::default();

    match engine.execute(action, None, "cli", &cfg) {
        Ok(receipt) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&receipt).unwrap());
            } else {
                println!("[+] Clean executed successfully: {}", action.label());
                println!("    Duration:         {} ms", receipt.duration_ms);
                println!("    Available Before: {} MB", receipt.available_before_mb);
                println!("    Available After:  {} MB", receipt.available_after_mb);
                println!("    Commit Before:    {} MB", receipt.commit_before_mb);
                println!("    Commit After:     {} MB", receipt.commit_after_mb);
                println!("    Receipt logged:   %LOCALAPPDATA%\\Headroom\\receipts.jsonl");
            }
            exit(0);
        }
        Err(EngineError::CooldownActive) => {
            if json {
                println!(r#"{{"status": "aborted", "reason": "cooldown_active"}}"#);
            } else {
                eprintln!("[-] Action aborted: Cooldown is currently active for this action.");
            }
            exit(1);
        }
        Err(EngineError::Busy) => {
            if json {
                println!(r#"{{"status": "aborted", "reason": "engine_busy"}}"#);
            } else {
                eprintln!("[-] Action aborted: Another cleaning action is currently running.");
            }
            exit(1);
        }
        Err(EngineError::IpcFailed(msg)) => {
            if json {
                println!(r#"{{"status": "error", "reason": "{}"}}"#, msg);
            } else {
                eprintln!("[-] IPC Error: {}", msg);
                eprintln!("    Ensure the Headroom Broker service is running in your VM.");
            }
            exit(3);
        }
        Err(e) => {
            if json {
                println!(r#"{{"status": "error", "reason": "{}"}}"#, e);
            } else {
                eprintln!("[-] Clean failed: {}", e);
            }
            exit(1);
        }
    }
}
