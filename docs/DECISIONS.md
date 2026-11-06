# Headroom Decision Log

This log records architectural decisions, API references, documentation URLs, and rationales according to Section 4 of the engineering brief.

---

## ADR-0001: Clean-Room MIT License & Zero-GPL Policy
- **Date**: 2026-11-05
- **Status**: Accepted
- **Context**: Mem Reduct is licensed under GPL-3.0. Headroom is licensed under MIT.
- **Decision**: No code from `henrypp/memreduct` or any GPL project will be read, copied, adapted, or incorporated. All logic is designed clean-room from public Windows documentation, Microsoft Learn, and empirical experiments.
- **References**:
  - MIT License: https://opensource.org/licenses/MIT

---

## ADR-0002: Rust Workspace Layout and Safety Boundaries
- **Date**: 2026-11-05
- **Status**: Accepted
- **Context**: High memory pressure requires reliability without crash-on-OOM or undefined behavior. Unsafe code must be audited and isolated.
- **Decision**:
  - `crates/core`: `#![forbid(unsafe_code)]` and zero Windows API dependencies. Pure domain logic, policy decision functions, and receipts.
  - `crates/sys`: Windows API wrappers. All `unsafe` blocks are contained here, each annotated with `// SAFETY:` explaining why calling conditions are valid.
  - Undocumented APIs are quarantined strictly in `crates/sys/src/undocumented.rs`.
- **References**:
  - Rust Safety: https://doc.rust-lang.org/nomicon/safe-unsafe-relationships.html

---

## ADR-0003: UI Framework Selection
- **Date**: 2026-11-05
- **Status**: Accepted
- **Context**: The background agent must reside comfortably under 5 MB private bytes and 0.1% idle CPU.
- **Decision**: Native Win32 API using the official `windows` crate. Electron, WebView2, and WinUI 3 are excluded from the persistent agent. The dashboard window runs in an on-demand process that exits completely when closed, returning its working set to the OS.
- **References**:
  - Windows Crate: https://crates.io/crates/windows
  - Microsoft Learn (Win32): https://learn.microsoft.com/en-us/windows/win32/

---

## ADR-0004: Undocumented Memory Cleaning APIs
- **Date**: 2026-11-05
- **Status**: Accepted (Pending Phase 0 Empirical VM Verification)
- **Context**: Standby and modified page list purges are not exposed via documented Win32 APIs. They require NT internal interfaces.
- **Decision**: Use `NtSetSystemInformation` with information class 80 (`SystemMemoryListInformation`) dynamically resolved via `GetProcAddress("ntdll.dll", "NtSetSystemInformation")` with `SeProfileSingleProcessPrivilege`. Commands:
  - 2: `MemoryEmptyWorkingSets`
  - 3: `MemoryFlushModifiedList`
  - 4: `MemoryPurgeStandbyList`
  - 5: `MemoryPurgeLowPriorityStandbyList`
- **References**:
  - Geoff Chappell NT System Information: https://www.geoffchappell.com/studies/windows/km/ntoskrnl/api/ex/sysinfo/memory_list.htm
  - Process Hacker / System Informer reference data on SystemMemoryListInformation.


---

## Phase 2: Clean Engine, IPC Protocol & Broker

### ADR-0005: Fixed-Size Binary Named Pipe Protocol (`headroom-ipc`)
- **Date**: 2026-11-06
- **Status**: Accepted
- **Context**: Inter-process communication between unelevated clients (agent, CLI, dashboard) and the privileged broker service.
- **Decision**: 12-byte fixed-size binary request (`u32` magic, `u8` version, `u8` action, `u16` padding, `u32` optional PID), 18-byte fixed-size binary response (`u32` magic, `u8` version, `u8` padding, `u32` status, `u32` ntstatus, `u64` duration_us).
- **Rationale**: Eliminates string injection vulnerabilities, arbitrary command execution, path traversal, and dynamic allocation attacks. Fuzzed against 10,000 randomized binary permutations.

### ADR-0006: Broker Security Hardening & Rate Limiting (`headroom-broker`)
- **Date**: 2026-11-06
- **Status**: Accepted
- **Context**: Running as a high-integrity service on Windows.
- **Decision**: Restrict pipe access with `PIPE_REJECT_REMOTE_CLIENTS`, `FILE_FLAG_FIRST_PIPE_INSTANCE`, and enforce a mandatory 15-second cooldown in the broker itself against repeated destructive actions (`PurgeStandby`, `FlushModifiedList`, `EmptyWorkingSets`).
- **Rationale**: Mitigates local Denial-of-Service and thrashing from unprivileged callers.

### ADR-0007: Receipts Audit Logging (`headroom-engine`)
- **Date**: 2026-11-06
- **Status**: Accepted
- **Context**: Requirement to prove cleaning results and disprove "placebo cleaner" claims.
- **Decision**: Append-only JSON Lines format written to `%LOCALAPPDATA%\Headroom\receipts.jsonl`.
- **Rationale**: Versioned schema with before/after memory, duration, and readback metrics provides an auditable receipt for every automated or manual action.


---

## Phase 3: Policy, Modes, History & Benchmark

### ADR-0008: Pure Functional Policy Engine (`crates/core`)
- **Date**: 2026-11-06
- **Status**: Accepted
- **Context**: Autonomous and recommendation logic for Headroom.
- **Decision**: Implemented as pure function `decide(&sample, &cfg, &state, is_fullscreen, foreground_pid) -> PolicyDecision`. Precedence:
  1. Commit Risk: Warn, list top committers, NEVER clean.
  2. Available RAM low: Trim background apps (protect foreground window, skip exclusions).
  3. Free low, standby large & Stutter Profile active: Purge low-priority standby.
  4. Otherwise: Do nothing.
- **Rationale**: Strict precedence guarantees zero-harm safety. Pure functional architecture ensures full testability against recorded fixtures without mock objects or OS side effects.

### ADR-0009: Pre-Allocated In-Memory History Ring Buffer (`crates/core`)
- **Date**: 2026-11-06
- **Status**: Accepted
- **Context**: 24-hour telemetry trend display without increasing agent footprint.
- **Decision**: Pre-allocated fixed dual-ring buffer (600 slots at 1-sec resolution for 10 min, 1440 slots at 1-min resolution for 24h) occupying under 50 KB total RAM.
- **Rationale**: Zero heap allocations at runtime ensures agent stability even in near-OOM conditions.

### ADR-0010: Locked Benchmark Methodology (`docs/BENCHMARK.md`)
- **Date**: 2026-11-06
- **Status**: Accepted
- **Context**: Verifying whether aggressive memory cleaning yields tangible benefits or causes performance regressions.
- **Decision**: Locked pass criteria: median app relaunch time and total hard faults within 5% of plain Windows across 4 GB and 8 GB VMs. Automatic mode remains experimental if this threshold is not met.
- **Rationale**: Prevents post-hoc metric tuning and ensures claims remain strictly evidence-based.
