# Headroom (v1.0)

[![CI](https://github.com/redwan2003-bot/HeadRoom/actions/workflows/ci.yml/badge.svg)](https://github.com/redwan2003-bot/HeadRoom/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-0078D6.svg)](https://microsoft.com/windows)

Headroom is an open-source, evidence-based Windows memory diagnostic and management utility designed for low-RAM PCs and gamers.

> [!NOTE]
> **Provenance & Clean-Room Guarantee**: Headroom was inspired by the utility of tools like Mem Reduct, but contains **none of its code** (100% clean-room MIT implementation; zero GPL code). As with any newly released open-source binary without commercial code-signing certificates, Windows SmartScreen may display a standard unrecognized publisher prompt on initial launch. See [`docs/SIGNING.md`](docs/SIGNING.md).

---

## Why Headroom?

Most RAM cleaners boast large "freed memory" numbers by aggressively purging Windows standby caches. This creates a harmful placebo effect: while physical free RAM looks higher, cached file pages are discarded, causing severe disk thrashing and app relaunch stutters as Windows is forced to re-read evicted data from disk.

Headroom takes a radically different, evidence-based approach:
1. **Diagnose**: Distinguishes true memory exhaustion from healthy system caching (Standby list).
2. **Conservative Action**: Default mode is **Monitor Only**. It never touches memory unless you opt in.
3. **Safety Invariants**: Never cleans under Commit Risk (which cannot be solved by trimming) and unconditionally pauses during fullscreen games.
4. **Verifiable Receipts**: Emits an audit receipt (`receipts.jsonl`) for every clean showing memory freed, commit deltas, and the subsequent 60-second disk read-back impact.

---

## Architecture & Binary Suite

```text
User session (Unelevated)                Windows Service / Helper
  Tray Agent  ───────┐                      Broker Service
  Dashboard   ───────┼─── Named Pipe ─────►   (Privileged SeProfileSingleProcessPrivilege)
  CLI Tool    ───────┘    (Fixed 12-byte)       │
       │                                        ▼
       ▼                                  NtSetSystemInformation (Class 80)
  Win32 / PSAPI Telemetry                 (Standby purge, working sets)
```

| Binary | Role | Executable Size |
|---|---|---|
| `headroom-agent.exe` | Persistent, ultra-lightweight tray agent (< 1.5 MB private RAM, 0% CPU) | **387.5 KB** |
| `headroom-dashboard.exe` | On-demand diagnostic window with dark theme and process breakdowns (< 250 ms launch) | **295.0 KB** |
| `headroom-cli.exe` | Scriptable command-line tool with `--json` output and stable exit codes | **337.5 KB** |
| `headroom-broker.exe` | Windows service helper for elevated NT memory operations in VM environments | **258.0 KB** |

---

## Installation & Portable Mode

### Portable Mode (No Installation Needed)
Simply copy the binaries into any folder. If a file named `portable` or `headroom.json` exists in the folder, Headroom runs in standalone portable mode, storing configuration and receipts locally without touching `%LOCALAPPDATA%`.

### User-Level Clean Installation (No Admin Rights Required)
To install Headroom into `%LOCALAPPDATA%\Programs\Headroom` with Start Menu integration:
```powershell
powershell -ExecutionPolicy Bypass -File dist\install.ps1
```

### Complete Clean Uninstallation
To cleanly uninstall Headroom and leave zero files or registry keys behind:
```powershell
powershell -ExecutionPolicy Bypass -File dist\uninstall.ps1 -PurgeUserData
```

### Windows Package Manager (Winget)
```powershell
winget install Headroom.Headroom
```

---

## Command-Line Usage

```powershell
# Query current diagnostic state
headroom status

# Output machine-readable JSON
headroom status --json

# Trigger clean action
headroom clean standby-low --json
```

---

## Documentation

- [`docs/USER_GUIDE.md`](docs/USER_GUIDE.md): Comprehensive guide to memory states, modes, and receipt analysis.
- [`docs/SECURITY.md`](docs/SECURITY.md): Threat modeling, named pipe security descriptors, and validation.
- [`docs/BENCHMARK.md`](docs/BENCHMARK.md): Workload methodologies, empirical test harnesses, and locked pass criteria.
- [`docs/DECISIONS.md`](docs/DECISIONS.md): Architectural Decision Records (ADR-0001 through ADR-0010).
- [`docs/SIGNING.md`](docs/SIGNING.md): SignPath Foundation application template for Authenticode signing.

---

## Quality & Standards

Every milestone passes the mandatory quality test suite:
```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

---

## License

Licensed under the [MIT License](LICENSE).
