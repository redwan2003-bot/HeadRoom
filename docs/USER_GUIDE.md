# Headroom User Guide & Documentation

Welcome to **Headroom**, an evidence-based memory diagnostic and management utility for Windows 10 and 11.

---

## 1. Core Philosophy: Why Free RAM is Not the Goal

On modern Windows, "free RAM" is wasted RAM. Windows utilizes unused memory as a **Standby Cache** to keep frequently accessed applications, DLLs, and files ready in memory.
- **Available RAM = Free + Zeroed + Standby**.
- When you use another memory cleaner that reports "Reclaimed 6 GB of RAM!", it simply evicted your file cache into the Free list. The next time you open an app, Windows must read those files from your physical disk again, causing system micro-stutters and disk thrashing.

Headroom is built to tell the truth about your memory and only intervene when it genuinely benefits performance.

---

## 2. The Four Memory States

Headroom continuously analyzes physical memory and commit charge, classifying the system into four states:

### 1. Healthy (Normal)
- **What it means**: The system has ample available memory. Applications and games run without paging pressure.
- **Action**: None needed.

### 2. Cache Full (Normal)
- **What it means**: Windows has cached gigabytes of files in Standby memory. Physical RAM may look "90% used", but it is actually fully available for apps whenever needed.
- **Action**: None. Windows automatically frees standby pages instantly when a program demands RAM. Purging this cache hurts performance.

### 3. Under Pressure (Attention Recommended)
- **What it means**: True physical available memory has dropped below safe working thresholds (typically < 10% or < 800 MB).
- **Action**: Trimming large, idle background applications allows active foreground tasks to breathe.

### 4. Commit Risk (Critical Warning)
- **What it means**: System-wide virtual memory allocations (Commit Charge) are nearing the commit limit (RAM + Pagefile).
- **Action**: **NEVER CLEAN**. Trimming working sets does not reduce commit charge. Headroom alerts you and identifies the top committers so you can save work before applications crash with Out-Of-Memory errors.

---

## 3. Operational Modes

Headroom provides three operating modes, selectable via the system tray icon right-click menu:

| Mode | Description | Default |
|---|---|---|
| **Monitor Only** | Passive diagnostics and tray status. **Never takes any cleaning action.** | **Yes** |
| **Ask First** | Recommends actions via notifications when pressure is detected, requiring your explicit click. | No |
| **Automatic Clean** | Autonomously trims eligible background applications under pressure, pausing during fullscreen games and respecting cooldowns. | No (Opt-in) |

---

## 4. Gamer Stutter Profile

Windows games that stream textures from disk can experience micro-stutter if the Free page list is completely exhausted and the OS must rapidly purge standby pages on the fly.

When **Stutter Profile** is enabled:
- If Free memory drops below 250 MB while Standby memory exceeds 2 GB, Headroom issues a privileged low-priority standby purge (`MemoryPurgeLowPriorityStandbyList`).
- This replenishes the Free list while preserving high-priority system caches.

---

## 5. Audit Receipts (`receipts.jsonl`)

Every cleaning action (whether triggered manually, via CLI, or by the policy) writes an append-only JSON receipt to:
`%LOCALAPPDATA%\Headroom\receipts.jsonl` (or `./receipts.jsonl` in portable mode).

### Example Receipt
```json
{
  "schema_version": 1,
  "timestamp_utc": 1793965619,
  "action": "Trim Process Working Set",
  "trigger": "manual_tray",
  "status": "success",
  "duration_ms": 12,
  "available_before_mb": 6126,
  "available_after_mb": 6540,
  "commit_before_mb": 11663,
  "commit_after_mb": 11663,
  "baseline_pagein_rate": 0.0,
  "readback_60s_estimate": 0.0,
  "confidence_note": "Read-back estimate is system-wide across all processes over 60 seconds."
}
```

Notice:
- `commit_before_mb` and `commit_after_mb` are identical (confirming the commit invariant).
- `readback_60s_estimate` measures whether subsequent disk faults increased due to the clean.

---

## 6. Command-Line Interface (CLI)

The `headroom-cli.exe` binary provides scriptable diagnostics and automation without any UI windows:

```powershell
# Show human-readable diagnostic status
headroom status

# Output machine-readable JSON status
headroom status --json

# Trigger cleaning actions (requires running broker or elevation)
headroom clean standby-low --json
headroom clean standby-all --json
headroom clean modified --json
headroom clean working-sets --json
```

### Stable Exit Codes
- `0`: Success
- `1`: Aborted by policy / Commit risk
- `2`: Invalid arguments
- `3`: Broker IPC unavailable
- `4`: Execution timeout
