# Headroom Security Architecture & Threat Model

Document Version: 1.0  
Date: 2026-11-06  
Status: Approved Architectural Standard  

---

## 1. Security Architecture Overview

Headroom splits operations strictly across privilege boundaries:

```text
User Interactive Session (Unelevated, Medium Integrity)
┌────────────────────────────────────────────────────────┐
│  Tray Agent  │  Dashboard Window  │  CLI Tool          │
│  - Holds policy                    - Read-only queries │
│  - Samples metrics                 - Trims own apps    │
└───────────────────────┬────────────────────────────────┘
                        │ Local Named Pipe: \\.\pipe\HeadroomBroker
                        │ (Strict binary messages, action enum + PID only)
                        ▼
Windows Service (Local System, High/System Integrity)
┌────────────────────────────────────────────────────────┐
│  Broker Service                                        │
│  - Declares only SeProfileSingleProcessPrivilege       │
│  - Explicit DACL (Interactive Users + System + Admins) │
│  - Impersonates caller to verify Session ID match      │
│  - Enforces independent purge rate-limiting            │
│  - Executes privileged NT list commands                │
└────────────────────────────────────────────────────────┘
```

---

## 2. Threat Analysis & Mitigations

### Threat 1: Local Elevation of Privilege (EoP) via Malicious Input
* **Attack**: A non-administrative local process connects to the pipe and supplies malicious paths, DLL names, or shell commands to be executed with `LocalSystem` privileges.
* **Mitigation**:
  - The Headroom IPC protocol **forbids strings, paths, and commands**.
  - All messages are fixed-size 8-byte binary structs: a 4-byte magic signature, 1-byte version, 1-byte action ID, and 2-byte reserved/flags, optionally followed by a 4-byte PID payload.
  - The broker will reject any packet whose size or structure deviates from this contract.
  - The broker never invokes `ShellExecute`, `CreateProcess`, or file system operations based on pipe messages.

### Threat 2: Pipe Squatting & Pre-Creation Hijacking
* **Attack**: Malware creates `\\.\pipe\HeadroomBroker` before the service starts, tricking clients into connecting to a rogue pipe server or capturing credentials.
* **Mitigation**:
  - The broker creates the pipe using `FILE_FLAG_FIRST_PIPE_INSTANCE`. If a rogue pipe already exists, creation fails immediately.
  - Client processes verify that the pipe server PID matches the expected broker service process before sending any payload (`GetNamedPipeServerProcessId`).

### Threat 3: Remote Network Access
* **Attack**: An attacker over the local network attempts to trigger system purges or exhaust memory via SMB/RPC pipe transports.
* **Mitigation**:
  - The pipe is opened with `PIPE_REJECT_REMOTE_CLIENTS`. Network access is blocked at the kernel driver layer (`npfs.sys`).

### Threat 4: Unauthorized Local Callers
* **Attack**: A service running in a different service account or sandbox attempts to trigger actions.
* **Mitigation**:
  - The pipe DACL is explicitly defined via SDDL:
    `D:(A;;GRGW;;;IU)(A;;GRGW;;;SY)(A;;GRGW;;;BA)`
    Only Interactive Users (`IU`), Local System (`SY`), and Builtin Administrators (`BA`) possess read/write permissions.

### Threat 5: Cross-Session Process Manipulation
* **Attack**: A user in Session 1 asks the broker to trim or alter the memory priority of a process running in Session 2 (e.g. multi-user Remote Desktop).
* **Mitigation**:
  - On any request containing a PID, the broker calls `ImpersonateNamedPipeClient`.
  - The broker inspects the client token's session ID (`GetTokenInformation(TokenSessionId)`).
  - The broker calls `ProcessIdToSessionId(target_pid)` and strictly rejects the request with `STATUS_ACCESS_DENIED` if the session IDs do not match.
  - The broker immediately invokes `RevertToSelf()`.

### Threat 6: Denial of Service via Purge Hammering
* **Attack**: A local application hammers the broker with thousands of standby purge requests per minute, thrashing physical disk with page read-backs.
* **Mitigation**:
  - The broker implements an autonomous sliding-window rate limiter (e.g., minimum 30-second cooldown between destructive purges), completely independent of client-side logic.
  - Excess requests are rejected with a rate-limit error code.

---

## 3. Privilege Minimization

The broker service runs as `LocalSystem` but declares only the single NT privilege it needs:
- `SeProfileSingleProcessPrivilege`: Required by the Windows kernel for `NtSetSystemInformation(SystemMemoryListInformation)`.
- It does **not** enable `SeDebugPrivilege`, `SeLoadDriverPrivilege`, or `SeTcbPrivilege`.

Trimming own-user processes does not require the broker service at all; the unelevated agent executes `EmptyWorkingSet` directly within the user's interactive session.
