# Headroom Benchmark Specification

**Status**: Published for Gate 3 Verification | **Version**: 1.0  
**Locked Decision**: The pass criteria and methodology defined herein cannot be modified after running benchmarks.

---

## 1. Objective

Many traditional memory cleaning utilities claim performance improvements based solely on a single cosmetic metric: "free RAM percentage". However, in modern virtual memory architectures, available RAM includes both free pages and cached standby pages. Purging the standby cache or aggressively trimming working sets often degrades system performance by converting fast in-memory page hits into costly disk I/O operations (hard page faults).

The purpose of this benchmark suite is to objectively and empirically measure the performance impact of Headroom compared against:
1. **Plain Windows (Baseline)**: Untouched Windows memory manager.
2. **Mem Reduct 3.5.2**: Official public binary from GitHub (GPL-3.0; treated strictly as an external black-box executable).
3. **Headroom (Monitor Mode)**: Passive diagnosis only.
4. **Headroom (Automatic Mode)**: Autonomous background trimming with foreground protection and adaptive cooldown.
5. **Headroom (Stutter Profile)**: Selective low-priority standby purging for low-free scenarios.

---

## 2. Test Environments

Benchmarks must be conducted in isolated Virtual Machines with clean snapshots to ensure repeatability:

| Environment | OS Version | vCPU | RAM | Storage |
|---|---|---|---|---|
| **VM-4G** (Low-RAM) | Windows 10 22H2 x64 | 2 vCPUs | 4,096 MB | 60 GB NVMe / SSD |
| **VM-8G** (Mainstream) | Windows 11 23H2 x64 | 4 vCPUs | 8,192 MB | 60 GB NVMe / SSD |

Both environments run with default pagefile sizing (system-managed) and UAC enabled.

---

## 3. Workloads

### Workload A: Cold App Relaunch & Soft-Fault Penalty
- **Description**: Simulates the user experience of switching back to apps after a cleaner has executed.
- **Procedure**:
  1. Boot VM to desktop and wait 120 seconds for boot idle.
  2. Launch 5 common desktop applications (Notepad, PowerShell, Paint, Edge, Windows Terminal) and populate with initial data.
  3. Execute cleaner action (or idle baseline).
  4. Minimize and relaunch each application sequentially.
  5. Measure wall-clock duration from launch request to active window focus, plus total hard page faults over the subsequent 60 seconds.
  6. Run 10 iterations per candidate.

### Workload B: Standby Cache Eviction & Read-Back Penalty
- **Description**: Measures disk thrashing caused by purging cached file pages.
- **Procedure**:
  1. Stream a 2.5 GB test file sequence into system standby cache.
  2. Trigger memory cleaner action (e.g. Purge Standby vs Headroom background trim).
  3. Re-read the file sequence.
  4. Measure disk throughput (MB/s), total hard faults, and completion latency.
  5. Run 10 iterations per candidate.

---

## 4. Metrics & Counters

| Metric | Source Counter / API | Unit | Interpretation |
|---|---|---|---|
| **Relaunch Latency** | High-resolution stopwatch (`QueryPerformanceCounter`) | Milliseconds (ms) | Lower is better. |
| **Hard Page Faults** | `\Memory\Pages Input/sec` (PDH) | Pages/sec | Lower is better. |
| **Total Disk Read-Back** | `\LogicalDisk(_Total)\Disk Read Bytes/sec` (PDH) | Megabytes (MB) | Lower is better. |
| **Available RAM** | `GetPerformanceInfo` -> `PhysicalAvailable` | Megabytes (MB) | Diagnostic context. |
| **Commit Charge** | `GetPerformanceInfo` -> `CommitTotal` | Megabytes (MB) | Verifies invariant. |

---

## 5. Locked Pass Criteria (Section 10)

For Headroom to pass Gate 3:
1. **App Relaunch Latency**: Median application relaunch time under Headroom must be **no more than 5% worse** than plain Windows on both 4 GB and 8 GB configurations.
2. **Hard Page Faults**: Total hard page faults over the 60-second observation window under Headroom must be **no more than 5% worse** than plain Windows.
3. **Commit Invariant**: Trimming must show **zero reduction in commit charge** (confirming that commit exhaustion cannot be solved by working set trimming).

> [!IMPORTANT]
> If Headroom Automatic Mode fails the 5% threshold, it will not be enabled by default. It will remain strictly experimental and hidden behind an advanced setting. The benchmark results will be published truthfully without post-hoc tuning.

---

## 6. Execution Instructions

The benchmark scripts are located in `bench/`:
- `bench/workload_relaunch.ps1`: App launch & relaunch timing harness.
- `bench/workload_cache.ps1`: File cache read-back thrash harness.
- `bench/run_matrix.bat`: Automated test harness orchestrating baseline, Mem Reduct, and Headroom runs.

Raw benchmark receipts will be saved to `bench/results/*.json`.
