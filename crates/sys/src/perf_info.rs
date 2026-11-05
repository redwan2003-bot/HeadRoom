//! Wrapper around documented PSAPI GetPerformanceInfo.

use crate::error::SysError;
use headroom_core::types::CommitInfo;
use std::mem::{size_of, zeroed};
use windows::Win32::Foundation::GetLastError;
use windows::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};

/// Snapshot of performance numbers retrieved from PSAPI.
#[derive(Debug, Clone, Copy)]
pub struct SystemPerformanceSnapshot {
    pub commit: CommitInfo,
    pub physical_total_bytes: u64,
    pub physical_available_bytes: u64,
    pub kernel_total_bytes: u64,
}

/// Retrieves global memory commit and physical numbers using GetPerformanceInfo.
pub fn query_performance_info() -> Result<SystemPerformanceSnapshot, SysError> {
    let mut perf_info: PERFORMANCE_INFORMATION = unsafe { zeroed() };
    let cb = size_of::<PERFORMANCE_INFORMATION>() as u32;
    perf_info.cb = cb;

    // SAFETY: GetPerformanceInfo is a documented Win32 API. Passing a valid, correctly-sized
    // pointer to PERFORMANCE_INFORMATION initialized with cb = sizeof(PERFORMANCE_INFORMATION).
    let res = unsafe { GetPerformanceInfo(&mut perf_info, cb) };

    if res.is_err() {
        let err = unsafe { GetLastError() };
        return Err(SysError::from_win32("GetPerformanceInfo", err.0));
    }

    let page_size = perf_info.PageSize as u64;

    Ok(SystemPerformanceSnapshot {
        commit: CommitInfo {
            total_bytes: (perf_info.CommitTotal as u64) * page_size,
            limit_bytes: (perf_info.CommitLimit as u64) * page_size,
            peak_bytes: (perf_info.CommitPeak as u64) * page_size,
        },
        physical_total_bytes: (perf_info.PhysicalTotal as u64) * page_size,
        physical_available_bytes: (perf_info.PhysicalAvailable as u64) * page_size,
        kernel_total_bytes: (perf_info.KernelTotal as u64) * page_size,
    })
}
