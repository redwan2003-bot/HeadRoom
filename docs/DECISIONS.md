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
