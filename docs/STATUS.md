# Headroom Project Status

Current Phase: **Phase 3 (Policy v0.3) — Complete & Awaiting Gate 3 Approval**
Last Updated: 2026-11-06

---

## Current Status Overview

- **Active Phase**: Phase 3: Policy, Modes, History & Benchmark (v0.3) (Complete)
- **Phase Goal**: Implement the pure policy engine (`decide()`), operating modes (`Monitor`, `AskFirst`, `Automatic`), gamer stutter profile, 24-hour circular history ring buffer, tray integration, and empirical benchmark specification & runner.
- **Current Stop Gate**: Gate 3. Awaiting explicit user approval: `approved, start Phase 4`.

---

## Progress Tracker

### Done
- [x] Phase 0 completed and verified with all 7 spikes.
- [x] Phase 1 completed and verified (tray agent, dashboard, classifier, PSAPI/PDH telemetry).
- [x] Phase 2 completed and verified (security threat model, binary IPC protocol, broker service, cleaning engine, CLI).
- [x] Task 3.1: `crates/core/src/policy.rs` pure decision function with strict precedence (Commit Risk -> Low Available -> Stutter Trigger -> None), foreground protection, and fullscreen gaming pauses.
- [x] Task 3.2: `crates/core/src/history.rs` pre-allocated dual-resolution circular ring buffer (10-min at 1s, 24-hr at 1m) under 50 KB total RAM with zero runtime heap allocations.
- [x] Task 3.3: 6 unit tests in `crates/core/tests/policy_tests.rs` verifying commit risk invariants, gaming pauses, foreground protection, and ring buffer rollover.
- [x] Task 3.4: Tray agent (`crates/agent`) integration with operational modes (`Monitor`, `AskFirst`, `Automatic`), gamer stutter profile toggle, and safe background execution threads.
- [x] Task 3.5: Benchmark specification in `docs/BENCHMARK.md` with locked pass criteria (<= 5% degradation compared to plain Windows).
- [x] Task 3.6: Workload scripts and VM test matrix runner in `bench/` (`workload_relaunch.ps1`, `workload_cache.ps1`, `run_matrix.bat`).

### In Progress
- [ ] Awaiting Gate 3 review and owner approval to proceed to Phase 4.

---

## Gate 3 Verification Matrix

| Component | Requirement | Verification Method | Status |
|---|---|---|---|
| **Pure Policy Logic** | Priority, foreground safe, gaming safe | `crates/core/src/policy.rs`, unit tests | **PASS [measured]** |
| **History Buffer** | Pre-allocated < 50 KB, 24-hour retention | `crates/core/src/history.rs`, unit tests | **PASS [measured]** |
| **Agent Executable Size**| Under 2 MB | `target/release/headroom-agent.exe` (**387.5 KB**) | **PASS [measured]** |
| **Dashboard Size** | Under 2 MB | `target/release/headroom-dashboard.exe` (**295.0 KB**) | **PASS [measured]** |
| **Benchmark Spec** | Formalized workload & pass criteria | `docs/BENCHMARK.md` | **PASS [documented]** |
| **Benchmark Harness** | Automated workload scripts & VM batch | `bench/` (`run_matrix.bat`, `.ps1` workloads) | **PASS [documented]** |

---

## Quality Command Verification
```powershell
cargo fmt --all --check                                # PASSED
cargo clippy --workspace --all-targets -- -D warnings   # PASSED (0 warnings)
cargo test --workspace                                # PASSED (15 tests across crates, 0 failures)
cargo deny check                                      # PASSED (advisories ok, bans ok, licenses ok, sources ok)
```
All checks passed on 2026-11-06.
