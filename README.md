# Headroom

Headroom is an open-source Windows memory diagnostic and management utility designed for low-RAM PCs and stutter-sensitive workloads.

> [!NOTE]
> Headroom was inspired by tools like Mem Reduct, but contains **none of its code** (100% clean-room MIT implementation). Unsigned or newly released binaries may cause Windows SmartScreen to display a warning.

---

## Philosophy & Core Goals

A higher free-RAM number proves nothing if Windows is merely caching useful data. Headroom focuses on:

1. **Diagnose**: Distinguish true memory exhaustion from healthy system caching (standby lists).
2. **Conservative Action**: Apply the least disruptive action only when genuine pressure demands it (default mode is monitor-only).
3. **Transparent Receipts**: Emit verifiable receipts for every clean event detailing memory freed and subsequent 60-second disk page-in penalty.

---

## Download & Releases

Pre-built binaries (including standalone portable `.exe` and installer packages) will be published under the [Releases](https://github.com/redwan2003-bot/Mem_Reduct/releases) section upon reaching release milestones (Phase 4 / v1.0).

---

## Building from Source

### Prerequisites
- Windows 10 (version 22H2+) or Windows 11 (x64 / ARM64).
- Rust 1.80+ (`stable-x86_64-pc-windows-msvc`).
- Microsoft C++ Build Tools / Visual Studio with Windows SDK.

### Build Command
```powershell
cargo build --release
```

Compiled binaries will be available in `target/release/`.

---

## Repository Structure

```text
headroom/
  AGENTS.md                  Engineering constraints and instructions
  Cargo.toml                 Workspace root configuration
  deny.toml                  License and dependency checking rules
  LICENSE                    MIT License
  README.md                  Project overview and documentation
  crates/
    core/                    Pure domain models, state classifier, policy, receipts (no unsafe, no Win32)
    sys/                     Safe wrappers over Win32/NT APIs (contains audited unsafe)
    engine/                  Action executors, cooldowns, and safety timeouts
    ipc/                     Named pipe protocol and message validators
    broker/                  Privileged Windows Service helper
    agent/                   Tray monitor agent (Win32)
    dashboard/               On-demand status and control window
    cli/                     Non-interactive CLI for scripting
  docs/
    PLAN.md                  Full project specification
    STATUS.md                Phase tracking and next steps
    DECISIONS.md             Architecture Decision Records (ADRs)
    SECURITY.md              IPC security model and threat analysis
    BENCHMARK.md             Empirical test harness and results
  spikes/                    Throwaway Phase 0 validation programs
  bench/                     Workloads and trace scripts
```

---

## License

Licensed under the [MIT License](LICENSE).
