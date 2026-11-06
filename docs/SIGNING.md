# SignPath Foundation Open-Source Code Signing Application

This document provides the prepared application and documentation for the project owner to submit Headroom to the **SignPath Foundation** (or Certum Open Source / SignPath Community Program) for free Authenticode code signing.

---

## 1. Project Information

- **Project Name**: Headroom
- **Repository URL**: `https://github.com/redwan2003-bot/HeadRoom`
- **License**: MIT License (OSI-Approved, clean-room implementation)
- **Primary Platform**: Windows 10 (22H2+) & Windows 11 (x64 and ARM64)
- **Primary Language & Stack**: Rust (`stable-x86_64-pc-windows-msvc`), official `windows` crate, Native Win32 only.
- **Project Lead / Owner**: Redwan Ahmmed (`redwan2003-bot`)

---

## 2. Project Description & Open-Source Purpose

Headroom is an open-source, evidence-based Windows memory diagnostic and management utility designed for users on resource-constrained PCs (4 GB – 8 GB RAM) and gamers seeking stutter elimination.

Unlike traditional "RAM booster" utilities that indiscriminately flush caches and induce severe disk thrashing, Headroom:
1. Distinguishes between healthy caching (Standby list) and true memory exhaustion.
2. Implements strict safety invariants: never interferes with active games or foreground windows.
3. Operates in Monitor-only mode by default.
4. Generates verifiable audit receipts (`receipts.jsonl`) showing exact memory reclaimed and subsequent 60-second read-back.

---

## 3. Security & Soundness Controls

Headroom is engineered according to non-negotiable security and clean-room requirements:
- **Clean-Room MIT Provenance**: 100% clean-room design. Zero code read or adapted from Mem Reduct (`henrypp/memreduct` - GPL 3.0) or any copyleft project.
- **Zero Telemetry**: Completely offline. No telemetry, analytics, network sockets, auto-updaters, or external requests.
- **Hardened Inter-Process Communication**: Fixed-size 12-byte binary messaging over named pipes with explicit security descriptors (`PIPE_REJECT_REMOTE_CLIENTS`), preventing unauthorized access or injection.
- **Memory Safety**: `crates/core` strictly enforces `#![forbid(unsafe_code)]`. All Win32 interop in `crates/sys` is isolated and annotated with explicit `// SAFETY:` soundness comments.
- **Supply Chain Security**: All dependencies are audited on every commit via `cargo deny` (licenses, bans, security advisories).

---

## 4. Why Authenticode Signing is Needed

Without Authenticode code signing:
1. **Windows SmartScreen Warnings**: New binary releases trigger "Windows protected your PC" (SmartScreen untrusted publisher warnings), deterring low-RAM users from running the diagnostic tools.
2. **UAC Prompts Show "Unknown Publisher"**: Elevated actions for portable mode display amber UAC dialogs rather than verified publisher credentials.
3. **Integrity Verification**: Code signing guarantees that distributed binaries have not been tampered with or modified post-build.

---

## 5. Submission Steps for Owner

1. Go to [SignPath Foundation Application](https://signpath.org/signpath-foundation/).
2. Submit the following details:
   - **Project Name**: Headroom
   - **Repository URL**: `https://github.com/redwan2003-bot/HeadRoom`
   - **Website / Docs**: Provide link to repository `README.md` and `docs/SECURITY.md`.
   - **Contact Email**: Project maintainer contact email.
3. Link the GitHub Actions workflow (`.github/workflows/ci.yml`) to SignPath for automated signing of release tags.
