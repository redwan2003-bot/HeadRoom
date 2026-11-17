# Release Notes: Headroom v1.1.0

**Headroom** is a lightweight, evidence-based, zero-telemetry Windows memory diagnostic and management utility engineered in Rust with native Win32 GDI.

Designed for low-RAM Windows PCs (4 GB – 8 GB) and stutter-sensitive gaming workloads, Headroom replaces cosmetic RAM flushes with principled kernel telemetry, transparent memory classification, and verified disk-thrash suppression.

---

## What's New in v1.1.0

### 1. Rolling GDI Mini-Sparkline Chart
- **10-Minute Rolling Trend**: Real-time visual polyline displaying RAM In-Use history across 600 seconds of sampling.
- **Visual Thresholds**: Clean 50% dashed horizontal guide, cyan memory trend line, and dynamic peak percentage tracking.
- **Zero Framework Bloat**: Painted purely with native Win32 GDI primitives (`CreatePen`, `Polyline`, `FillRect`), consuming under 40 KB of code and 0% CPU overhead.

### 2. Global Hotkey Daemon (`Ctrl + Shift + H`)
- **System-Wide Instant Access**: Spawns or brings the on-demand diagnostic dashboard to the foreground from anywhere in Windows via `Ctrl + Shift + H`.
- **Low Footprint Background Listener**: Integrated into `headroom-agent.exe` without polling loops.

### 3. Interactive Process Exclusion Manager
- **Per-Process Protection**: Protect foreground games, IDEs, and heavy applications from being trimmed.
- **Live GUI Toggle**: Interactive "Toggle Exclude Top App" action in the dashboard process list with visual `[EXCLUDED]` badges.
- **Automatic Configuration Persistence**: Saves exclusion rules locally to `%LOCALAPPDATA%\Headroom\config.json` (or `headroom.json` in portable mode).

### 4. Production Hardening Suite
- **Application Manifests & DPI Awareness**: Embedded manifests with Windows 10/11 compatibility GUIDs and `PerMonitorV2` high-DPI scaling across all executables (`assets/app.manifest`).
- **Elevated Syscall Fallback & Privilege Badge**: When run as Administrator, CLI and agent can execute memory list purges directly via NT syscalls; shows clear troubleshooting instructions when run unelevated without the broker service. Displays real-time `[Elevated / Service Active]` vs `[Standard / Portable]` badges in the UI.
- **Local Crash & Panic Forensics**: Integrated zero-telemetry panic handler logging local diagnostics to `crash.log`.
- **Responsive Resizing**: Added `WM_GETMINMAXINFO` enforcing a 680×600 minimum window dimension and dynamic `WM_SIZE` repaints.
- **Quiet In-App Update Checker**: Opt-in GitHub Releases query button with zero background network activity.
- **SmartScreen & Whitelisting**: Automated SignPath CI workflow (`.github/workflows/signpath.yml`) and Microsoft Defender WDSI submission packager (`dist/submit_wdsi.ps1`).

### 5. Multi-Channel Distribution
- **Microsoft Store (MSIX & Win32)**: AppxManifest, generated store assets, local packaging script (`dist/msstore/package_msix.ps1`), and automated CI workflow (`.github/workflows/msstore-build.yml`).
- **Windows Package Manager (Winget)**: Automated CI workflow (`.github/workflows/winget-publish.yml`) and valid v1.6.0 manifests in `dist/winget/`.
- **Portable ZIP**: Self-contained archive with automatic `headroom.json` portable detection.
- **Inno Setup Installer**: Minimal, zero-residue setup script (`dist/headroom.iss`).

---

## Measured Performance & Budgets

All metrics measured on 64-bit Windows systems:

| Metric | Target / Budget | Measured Result | Status |
|---|---|---|---|
| **Agent Executable Size** | < 2.0 MB | **425.0 KB** | PASS |
| **Dashboard Size** | < 2.0 MB | **400.5 KB** | PASS |
| **CLI Executable Size** | < 2.0 MB | **338.0 KB** | PASS |
| **Broker Service Size** | < 2.0 MB | **258.0 KB** | PASS |
| **Complete ZIP Distribution** | < 2.0 MB | **772.9 KB** | PASS |
| **Agent Private RAM** | < 5.0 MB | **1.31 MB** | PASS |
| **Agent Idle CPU** | < 0.1% | **0.0000%** | PASS |
| **Dashboard Startup Latency** | < 300 ms | **212.7 ms** | PASS |
| **External Telemetry Requests** | 0 | **0 (Strictly Local)** | PASS |

---

## Cryptographic Hashes (SHA-256)

```text
646d4cc0ca5d19b663c818cf1f08b0bd95efbb6a8eaa3634d8750a7ccde1de8c  Headroom-v1.1.0-windows-x64.zip
fb27ac67b9741fdaacc51a0ba5935542885fe473c4668413475c435688e61d34  headroom-agent.exe
3de58198ba8f5d246ce51f6d219db9c98402a5323df2c9cb8a98da6e2d3b9f3d  headroom-broker.exe
6cbf9ba740cce7b4ecb1119af95ff401c123916fbdca7706d94660b82b54776b  headroom-cli.exe
2c42b46f2acaa074c9fcd6f54da56db4822f77265415240ea76f43cf1a2600a9  headroom-dashboard.exe
5ff7c626fc9586443e961c2467563049d8649bb5e49d3e55b59e453ede136d30  Headroom-WDSI-Submission-v1.1.0.zip
```

---

## Installation & Usage

### Via Winget
```cmd
winget install Headroom.Headroom
```

### Portable Mode
Download `Headroom-v1.1.0-windows-x64.zip`, extract to any folder, and run `headroom-agent.exe` or `headroom-dashboard.exe`. To force portable configuration, place an empty `headroom.json` in the folder.

### Command Line
```cmd
headroom status           # Display live memory telemetry
headroom status --json    # Output telemetry as JSON for scripts
headroom clean standby-low # Purge low-priority standby cache (gamer profile)
```

---

## License & Provenance Notice
Headroom is licensed under the permissive **MIT License**. It was developed from first principles with zero code, assets, or patterns derived from any copyleft or GPL-licensed software.
