//! Safe wrapper over Performance Data Helper (PDH) English counters.

use crate::error::SysError;
use std::mem::zeroed;
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterValue,
    PdhOpenQueryW, PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_FMT_LARGE,
};

/// Performance counters bundle sampled together.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryCountersSnapshot {
    pub modified_bytes: u64,
    pub free_zero_bytes: u64,
    pub pages_input_per_sec: f64,
}

/// Lightweight PDH query holder maintaining handles to minimal essential counters.
pub struct PdhQueryManager {
    query: isize,
    cnt_modified: isize,
    cnt_free_zero: isize,
    cnt_pages_input: isize,
}

impl PdhQueryManager {
    /// Initializes a new PDH query and registers essential English memory counters.
    pub fn new() -> Result<Self, SysError> {
        let mut query: isize = 0;

        // SAFETY: PdhOpenQueryW is called with null data source (local machine) and 0 context.
        let status = unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) };
        if status != ERROR_SUCCESS.0 {
            return Err(SysError::Pdh {
                operation: "PdhOpenQueryW",
                status,
            });
        }

        let add_counter = |path: &str| -> Result<isize, SysError> {
            let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            let mut handle: isize = 0;

            // SAFETY: PdhAddEnglishCounterW ensures language-invariant counter resolution.
            let res = unsafe {
                PdhAddEnglishCounterW(query, PCWSTR::from_raw(wide.as_ptr()), 0, &mut handle)
            };
            if res != ERROR_SUCCESS.0 {
                unsafe {
                    let _ = PdhCloseQuery(query);
                };
                return Err(SysError::Pdh {
                    operation: "PdhAddEnglishCounterW",
                    status: res,
                });
            }
            Ok(handle)
        };

        let cnt_modified = add_counter(r"\Memory\Modified Page List Bytes")?;
        let cnt_free_zero = add_counter(r"\Memory\Free & Zero Page List Bytes")?;
        let cnt_pages_input = add_counter(r"\Memory\Pages Input/sec")?;

        // Prime the query with an initial collection
        unsafe {
            let _ = PdhCollectQueryData(query);
        }

        Ok(Self {
            query,
            cnt_modified,
            cnt_free_zero,
            cnt_pages_input,
        })
    }

    /// Collects latest query metrics and returns formatted sample.
    pub fn collect(&mut self) -> Result<MemoryCountersSnapshot, SysError> {
        // SAFETY: query handle is valid.
        let status = unsafe { PdhCollectQueryData(self.query) };
        if status != ERROR_SUCCESS.0 {
            return Err(SysError::Pdh {
                operation: "PdhCollectQueryData",
                status,
            });
        }

        let read_u64 = |handle: isize| -> u64 {
            let mut val: PDH_FMT_COUNTERVALUE = unsafe { zeroed() };
            let res = unsafe { PdhGetFormattedCounterValue(handle, PDH_FMT_LARGE, None, &mut val) };
            if res == ERROR_SUCCESS.0 {
                unsafe { val.Anonymous.largeValue.max(0) as u64 }
            } else {
                0
            }
        };

        let read_f64 = |handle: isize| -> f64 {
            let mut val: PDH_FMT_COUNTERVALUE = unsafe { zeroed() };
            let res =
                unsafe { PdhGetFormattedCounterValue(handle, PDH_FMT_DOUBLE, None, &mut val) };
            if res == ERROR_SUCCESS.0 {
                unsafe { val.Anonymous.doubleValue.max(0.0) }
            } else {
                0.0
            }
        };

        Ok(MemoryCountersSnapshot {
            modified_bytes: read_u64(self.cnt_modified),
            free_zero_bytes: read_u64(self.cnt_free_zero),
            pages_input_per_sec: read_f64(self.cnt_pages_input),
        })
    }
}

impl Drop for PdhQueryManager {
    fn drop(&mut self) {
        if self.query != 0 {
            // SAFETY: query is a valid PDH query handle being cleaned up.
            unsafe {
                let _ = PdhCloseQuery(self.query);
            }
        }
    }
}
