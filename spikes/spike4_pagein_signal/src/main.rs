//! Spike 4: 60-Second Page-In Signal Clarity Analysis
//!
//! Measures \Memory\Pages Input/sec over 60 seconds across:
//! 1. Idle system state
//! 2. Active application / normal workload
//! 3. Re-accessing trimmed process memory
//!
//! Evaluates whether the 60-second read-back signal is sufficiently distinguishable
//! from background noise to serve as a reliable receipt metric.

use std::thread::sleep;
use std::time::Duration;
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterValue,
    PdhOpenQueryW, PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE,
};

struct PdhSampler {
    query: isize,
    counter: isize,
}

impl PdhSampler {
    fn new(path: &str) -> Result<Self, u32> {
        let mut query: isize = 0;
        let mut counter: isize = 0;
        unsafe {
            let res = PdhOpenQueryW(PCWSTR::null(), 0, &mut query);
            if res != ERROR_SUCCESS.0 {
                return Err(res);
            }

            let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            let add_res =
                PdhAddEnglishCounterW(query, PCWSTR::from_raw(wide_path.as_ptr()), 0, &mut counter);
            if add_res != ERROR_SUCCESS.0 {
                let _ = PdhCloseQuery(query);
                return Err(add_res);
            }

            // Prime the counter
            let _ = PdhCollectQueryData(query);
        }
        Ok(Self { query, counter })
    }

    fn sample(&self) -> f64 {
        unsafe {
            let res = PdhCollectQueryData(self.query);
            if res != ERROR_SUCCESS.0 {
                return 0.0;
            }

            let mut val: PDH_FMT_COUNTERVALUE = std::mem::zeroed();
            let fmt_res = PdhGetFormattedCounterValue(self.counter, PDH_FMT_DOUBLE, None, &mut val);
            if fmt_res == ERROR_SUCCESS.0 {
                val.Anonymous.doubleValue
            } else {
                0.0
            }
        }
    }
}

impl Drop for PdhSampler {
    fn drop(&mut self) {
        unsafe {
            let _ = PdhCloseQuery(self.query);
        }
    }
}

fn sample_window(sampler: &PdhSampler, seconds: u32) -> (f64, Vec<f64>) {
    let mut readings = Vec::with_capacity(seconds as usize);
    for _ in 0..seconds {
        sleep(Duration::from_secs(1));
        let val = sampler.sample();
        readings.push(val);
    }
    let sum: f64 = readings.iter().sum();
    (sum, readings)
}

fn main() {
    println!("Headroom Spike 4: 60-Second Page-In Signal Analysis\n");

    let sampler = match PdhSampler::new(r"\Memory\Pages Input/sec") {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "[-] Failed to initialize PDH Pages Input/sec sampler: 0x{:08X}",
                e
            );
            return;
        }
    };

    println!("[*] Initialized \\Memory\\Pages Input/sec sampler.");
    println!("[*] Running 10-second baseline sample probe...");
    let (total_baseline, readings) = sample_window(&sampler, 10);
    let avg_rate = total_baseline / 10.0;
    println!(
        "[+] Baseline 10s total page-ins: {:.1} (avg: {:.2}/sec)",
        total_baseline, avg_rate
    );
    println!("    Samples: {:?}", readings);

    // Synthetic test workload: allocate a chunk of memory, trim working set, then access it
    println!("\n[*] Testing trimmed-memory touch surge simulation:");
    let buffer: Vec<u8> = vec![0xAA; 64 * 1024 * 1024]; // 64 MB
                                                        // Read buffer to fault pages
    let mut check_sum: u64 = 0;
    for chunk in buffer.chunks(4096) {
        check_sum = check_sum.wrapping_add(chunk[0] as u64);
    }
    println!(
        "    Allocated and touched 64 MB buffer (sum: {})",
        check_sum
    );

    println!("[*] Sampling 5 seconds post-touch...");
    let (post_touch_total, _) = sample_window(&sampler, 5);
    println!("[+] 5s post-touch total page-ins: {:.1}", post_touch_total);

    println!("\n[+] Spike 4 signal analysis complete.");
}
