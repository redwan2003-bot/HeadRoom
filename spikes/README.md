# Headroom Phase 0 Spikes

This directory contains throwaway research programs used to validate architectural and systems assumptions before implementing production crates. In accordance with Section 5 of the project brief, code in `spikes/` is never imported by `crates/`.

---

## Spike Directory Index

- `spike1_counters`: Evaluates unelevated memory counters via PDH (`PdhAddEnglishCounterW`), `GetPerformanceInfo`, and `NtQuerySystemInformation(80)`.
- `spike2_priv_actions`: Probe of privileged cleaning actions (Standby/Modified lists, Working Sets) and required NT privileges (`SeProfileSingleProcessPrivilege`). **Must be run inside a VM.**
- `spike3_commit_invariant`: Empirically tests whether working set trimming leaves system commit charge unchanged. **Must be run inside a VM.**
- `spike4_pagein_signal`: Measures 60-second `\Memory\Pages Input/sec` signal clarity across baseline and faulted states for clean receipt accuracy.
- `spike5_unelevated_trim`: Verifies whether unelevated processes can trim working sets (`EmptyWorkingSet`) and alter process memory priority (`SetProcessInformation(ProcessMemoryPriority)`) on own-user processes.
- `spike6_footprint`: Benchmarks tray agent and dashboard window instantiation latency, private memory footprint, and idle CPU usage.
- `spike7_fullscreen`: Tests `SHQueryUserNotificationState` and foreground window bounds to reliably detect fullscreen and borderless games.

---

## Running Spikes 2 & 3 in a VM

1. Build the release binaries on the host:
   ```powershell
   cargo build --release -p spike2_priv_actions -p spike3_commit_invariant
   ```
2. Copy `target/release/spike2_priv_actions.exe`, `target/release/spike3_commit_invariant.exe`, and `spikes/run_in_vm.bat` into a folder inside the test Windows VM.
3. Inside the VM, open an elevated command prompt (Run as Administrator) and execute `run_in_vm.bat`.
4. Copy the resulting `results/spike2.json` and `results/spike3.json` files back to `spikes/results/` on the development workstation.
