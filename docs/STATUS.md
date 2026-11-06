# Headroom Project Status

Current Phase: **Phase 2 (Clean v0.2) — Complete & Awaiting Gate 2 Approval**
Last Updated: 2026-11-06

---

## Current Status Overview

- **Active Phase**: Phase 2: Clean (v0.2) (Complete)
- **Phase Goal**: Implement the named pipe IPC protocol (`crates/ipc`), privileged broker service (`crates/broker`), cleaning engine with receipts logging (`crates/engine`), safe process wrappers (`crates/sys`), and scriptable CLI (`crates/cli`).
- **Current Stop Gate**: Gate 2. Awaiting explicit user approval: `approved, start Phase 3`.

---

## Progress Tracker

### Done
- [x] Phase 0 completed and verified with all 7 spikes.
- [x] Phase 1 completed and verified (tray agent, dashboard, classifier, PSAPI/PDH telemetry).
- [x] Task 2.1: `docs/SECURITY.md` authored with threat model, SDDL DACL, named pipe hardening, and validation specs.
- [x] Task 2.2: `crates/ipc` fixed-size binary message framing (12-byte request, 18-byte response), zero heap strings, 10,000-permutation parser fuzzer.
- [x] Task 2.3: `crates/sys/src/undocumented.rs` with `NtSetSystemInformation` (Class 80) wrapper for VM test execution; safe same-user process trimming and priority adjustment in `crates/sys/src/processes.rs`.
- [x] Task 2.4: `crates/engine` with one-at-a-time execution lock, action cooldown manager, and verifiable `CleanReceipt` logger to `%LOCALAPPDATA%\Headroom\receipts.jsonl`.
- [x] Task 2.5: `crates/broker` privileged service with `SeProfileSingleProcessPrivilege`, message-mode pipe, 15-second rate limiter for destructive purges, and console mode for VM validation.
- [x] Task 2.6: `crates/cli` non-interactive CLI supporting `status` (human + `--json`) and `clean <action>` with deterministic exit codes.
- [x] Task 2.7: VM runner script `spikes/run_gate2_vm.bat` for safe privileged action verification.

### In Progress
- [ ] Awaiting Gate 2 review and owner approval to proceed to Phase 3.

---

## Gate 2 Deliverables Matrix

| Component | Requirement | Verification Method | Status |
|---|---|---|---|
| **Security Architecture** | Threat model, DACL, pipe flags | `docs/SECURITY.md` | **PASS [documented]** |
| **IPC Protocol Framing** | Fixed binary structs, no strings | `crates/ipc`, 10k fuzzer test | **PASS [measured]** |
| **Receipts Audit Log** | Local JSONL logging with schema | `crates/engine/src/receipts.rs` | **PASS [measured]** |
| **CLI Status** | Human-readable & JSON format | `target/release/headroom-cli.exe` | **PASS [measured]** |
| **VM Verification Package** | Safe test batch for VM | `spikes/run_gate2_vm.bat` | **PASS [documented]** |

---

## Quality Command Verification
```powershell
cargo fmt --all --check                                # PASSED
cargo clippy --workspace --all-targets -- -D warnings   # PASSED (0 warnings)
cargo test --workspace                                # PASSED (all tests passed)
cargo deny check                                      # PASSED (advisories ok, bans ok, licenses ok, sources ok)
```
All checks passed on 2026-11-06.
