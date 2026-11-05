# AGENTS.md — Headroom Engineering Context

Full brief: [`docs/PLAN.md`](file:///e:/Mem_Reduct/docs/PLAN.md) | Current status: [`docs/STATUS.md`](file:///e:/Mem_Reduct/docs/STATUS.md) | Decisions: [`docs/DECISIONS.md`](file:///e:/Mem_Reduct/docs/DECISIONS.md)

---

## 1. Locked Project Decisions

- **Product Name**: Headroom (keep in one constant for low-friction renaming).
- **Target Audience**: Low-RAM Windows PC users; opt-in stutter profile for gamers.
- **Default Mode**: Monitor only; automatic cleaning is opt-in.
- **Language & Framework**: Rust with the official `windows` crate.
- **UI**: Native Win32 only. No Electron, webview, or WinUI 3 in the persistent agent.
- **License**: MIT clean-room.
- **Platforms**: Windows 10 22H2 and Windows 11 (x64 first, ARM64 by v1.0).
- **Telemetry**: Zero telemetry; all data remains local on the host PC.

---

## 4. Non-Negotiable Engineering Rules

### Accuracy & Verification
- Never invent an API, constant, crate feature, counter name, benchmark number, or test result.
- Verify Win32 and crate details against Microsoft Learn and docs.rs; record source URLs in comments or `docs/DECISIONS.md`.
- Label every claim in reports as **measured**, **documented**, or **assumed**. Include command line and raw output path for measured claims.
- Never state something works without running and verifying it. Successful compilation is not a test pass.
- If reality contradicts the brief, stop and report immediately.

### Stop Gates
- Stop at each gate in Section 10 and wait for the exact phrase: `approved, start Phase N`.
- If a failure survives two different fix attempts, stop and report options.

### Safety of Owner's PC
- **Never run memory-clean actions on the host PC** until the owner explicitly approves that specific action.
- Elevated or destructive tests run strictly inside a virtual machine. Provide a runnable script and instructions; read results from `spikes/results/` or `bench/results/`.
- Do not install services on the host; do not modify system settings, registry, power plans, or page files.
- Keep all file changes inside the workspace. Global tool installs require explicit owner approval.
- Never push to remote, publish a release, tag, or submit to winget without explicit approval.

### Legal & Clean-Room Rules
- Do NOT read, copy, or adapt code from `henrypp/memreduct` or any GPL project. (Mem Reduct is GPL-3.0; Headroom is MIT).
- Use only public documentation, release notes, issue titles, and self-authored experiments.
- Check dependency licenses via `cargo deny` (Allow: MIT, Apache-2.0, BSD, ISC, Zlib, Unicode).

### Code Standards
- `core`: `#![forbid(unsafe_code)]` and zero Windows dependencies.
- `sys`: Contains all `unsafe` blocks, each with a required `// SAFETY:` soundness comment.
- Undocumented Windows calls live exclusively in `crates/sys/src/undocumented.rs`.
- No `unwrap` or `expect` outside test code or verified startup invariants. Use typed errors with `thiserror`.
- No TODOs, placeholders, or stubs in merged code. Keep dependencies minimal and documented in `docs/DECISIONS.md`.

### Quality Command
Every task must pass before completion:
```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

### Resumable Work
Always keep `docs/STATUS.md` updated with phase, done, in progress, next step, open questions, and quality command status.
