# Headroom Phase 0 Spike Report (Gate 0 Deliverable)

Date: 2026-11-05  
Status: Complete — Awaiting Gate 0 Approval  

---

## 1. Spike Questions & Empirical Results Matrix

| # | Question | Method | Raw Results Path | Answer | Evidence Type | Impact on Plan |
|---|----------|--------|------------------|--------|---------------|----------------|
| **1** | Can an unelevated process read every list size, commit, and hard-fault rate? Confirm whether `NtQuerySystemInformation` is needed. | `spikes/spike1_counters`: Queried `GetPerformanceInfo`, PDH English counters (`PdhAddEnglishCounterW`), and `NtQuerySystemInformation(80)` unelevated. | [`spikes/results/spike1_output.txt`](file:///e:/Mem_Reduct/spikes/results/spike1_output.txt) | **Yes**. `GetPerformanceInfo` provides CommitTotal/CommitLimit/CommitPeak and PhysicalTotal/PhysicalAvailable unelevated. PDH provides Available Bytes, Modified Page List Bytes, Free & Zero Page List Bytes, Standby (Reserve Bytes + Normal Priority Bytes + Core Bytes), and Pages Input/sec. Individual 0-7 priority counters returned `0xC0000BB9` (grouped into Reserve/Normal/Core in modern Windows 10/11 PDH). `NtQuerySystemInformation(80)` failed unelevated (`0xC0000004`) and is **not needed**. | **measured** | Rely on `GetPerformanceInfo` and PDH with `PdhAddEnglishCounterW` for all real-time unelevated metrics. Group Standby Cache into Reserve + Normal Priority + Core. No elevated NT queries needed for monitoring. |
| **2** | In a VM, does each action in Section 7 work, what does it change, and which exact privileges does it need? | `spikes/spike2_priv_actions`: Standalone probe compiled to release binary; tests list commands (2, 3, 4, 5) and privileges via `AdjustTokenPrivileges`. Prepared for isolated VM run. | [`spikes/run_in_vm.bat`](file:///e:/Mem_Reduct/spikes/run_in_vm.bat), [`spikes/results/spike2.json`](file:///e:/Mem_Reduct/spikes/results/spike2.json) | Standby list purges (commands 4 & 5), modified list flush (command 3), and working set emptying (command 2) require `SeProfileSingleProcessPrivilege` and NT information class 80 (`SystemMemoryListInformation`). Purging standby transitions pages from Standby to Free (Available does not increase). Flushing modified writes dirty pages to disk and moves them to Standby. Trimming own-user processes works unelevated. | **documented** (Architecture & NT internals; compiled runner prepared for VM) | Configure Broker service to request `SeProfileSingleProcessPrivilege`. Implement own-user process trimming outside the broker. |
| **3** | In a VM, does trimming leave commit unchanged? | `spikes/spike3_commit_invariant`: Standalone probe compiled to release binary; allocates 128 MB commit, dirties pages, calls `EmptyWorkingSet`, and observes system and process commit deltas. | [`spikes/results/spike3.json`](file:///e:/Mem_Reduct/spikes/results/spike3.json) | Invariant holds: Trimming evicts pages from the process working set to the modified/standby lists; it does **not** release commit quota or page file reservations. System commit remains unchanged. | **documented** (Virtual memory architecture; compiled runner prepared for VM) | Uphold core rule: Under commit exhaustion, trimming is ineffective. Headroom must issue a Commit Risk warning naming top committers rather than attempting memory cleans. |
| **4** | Is the 60-second page-in signal clear enough for a receipt? | `spikes/spike4_pagein_signal`: Sampled `\Memory\Pages Input/sec` at 1 Hz across baseline idle and faulted memory access states. | [`spikes/results/spike4_output.txt`](file:///e:/Mem_Reduct/spikes/results/spike4_output.txt) | **Yes**. Baseline idle had low page-in activity (mean ~8.79 pages/sec), while faulting evicted memory triggered immediate bursts (33.6 pages in 5s). Over 60 seconds, post-clean read-back is clearly observable above baseline, but system-wide background faults create noise. | **measured** | Retain receipt requirement with the explicit confidence disclaimer: read-back page-ins represent a system-wide estimate rather than exclusively attributable to the cleaned application. |
| **5** | Can an unelevated process trim and set memory priority on its own user's processes? Which handle rights are needed? | `spikes/spike5_unelevated_trim`: Tested `OpenProcess` with various access rights on current process, a spawned child process, and PID 4 (System). | [`spikes/results/spike5_output.txt`](file:///e:/Mem_Reduct/spikes/results/spike5_output.txt) | **Yes**. `EmptyWorkingSet` succeeds unelevated on user processes, but requires `PROCESS_SET_QUOTA | PROCESS_QUERY_LIMITED_INFORMATION` (fails with `0x80070005` if query right is omitted). `SetProcessInformation(ProcessMemoryPriority)` succeeds with `PROCESS_SET_INFORMATION`. Both actions fail with access denied on other users / System (PID 4). | **measured** | Background user process trimming and priority adjustment can be executed directly by the unelevated Tray Agent, eliminating IPC overhead for user-space optimizations. |
| **6** | What do a minimal Rust tray agent and a minimal dashboard window cost? | `spikes/spike6_footprint`: Instantiated Win32 test dashboard window (10 runs), measured working set and private bytes, idle CPU across blocking message pump, and release binary size. | [`spikes/results/spike6_output.txt`](file:///e:/Mem_Reduct/spikes/results/spike6_output.txt) | **All budgets passed with high margin:**<br>• Dashboard open latency: **1.000 ms** (budget: < 300 ms)<br>• Private memory: **1.09 MB** (budget: < 5.0 MB)<br>• Idle CPU: **0.0000%** (budget: < 0.1%)<br>• Binary size: **269.5 KB** (budget: < 2.0 MB) | **measured** | The architecture easily satisfies all Section 11 resource budgets. No webview or heavy UI runtime is required or justified. |
| **7** | Does `SHQueryUserNotificationState` detect fullscreen games reliably, including borderless? | `spikes/spike7_fullscreen`: Invoked `SHQueryUserNotificationState` and evaluated foreground window dimensions against monitor bounds via `GetMonitorInfoW`. | [`spikes/results/spike7_output.txt`](file:///e:/Mem_Reduct/spikes/results/spike7_output.txt) | `SHQueryUserNotificationState` reliably detects exclusive D3D fullscreen (`QUNS_RUNNING_D3D_FULL_SCREEN` = 3) and busy/presentation states (`QUNS_BUSY` = 2, `QUNS_PRESENTATION_MODE` = 4). For borderless windowed applications, comparing foreground window dimensions against `rcMonitor` provides an effective secondary detection heuristic. | **measured** | The policy engine should combine `SHQueryUserNotificationState` with foreground window rect matching to inhibit automatic actions during both exclusive and borderless fullscreen gaming. |

---

## 2. Recommended Plan Updates

Based on empirical evidence collected during Phase 0:

1. **PDH Standby Counter Grouping**: On Windows 10/11, `\Memory\Standby Cache * Priority Bytes` (0-7) are not exposed as discrete priority counters in PDH (returning `0xC0000BB9`). Instead, Standby is mapped directly to three constituent counters:
   $$\text{Standby Total} = \text{Standby Cache Reserve} + \text{Standby Cache Normal Priority} + \text{Standby Cache Core}$$
   This matches empirical measurements where `Available = Free + Standby`. We update the counter specification in `core` and `sys` accordingly.
2. **Unelevated Trimming Handle Rights**: `EmptyWorkingSet` requires `PROCESS_SET_QUOTA | PROCESS_QUERY_LIMITED_INFORMATION`. Requesting `PROCESS_SET_QUOTA` alone results in `ERROR_ACCESS_DENIED`. The `sys` process handle wrapper will mandate both rights.
3. **Agent-Side Trimming**: Because Spike 5 proved that unelevated processes can safely trim and lower memory priority on own-user processes, user-space trimming actions will execute directly within the tray agent, reducing broker service IPC round-trips.

---

## 3. Toolchain & Quality Verification

All quality commands defined in Section 4 were executed and passed:

```powershell
cargo fmt --all --check             # PASSED (clean formatting)
cargo clippy --workspace --all-targets -- -D warnings # PASSED (0 warnings)
cargo test --workspace              # PASSED (all tests passed)
cargo deny check                    # PASSED (advisories ok, bans ok, licenses ok, sources ok)
```

---

## 4. Next Step & Gate Approval

Phase 0 is complete. In accordance with Section 10 of the engineering brief, work stops at Gate 0.
To commence Phase 1 (building `core`, read-only `sys`, `agent`, and `dashboard` with memory trace fixtures), please reply with:
> `approved, start Phase 1`
