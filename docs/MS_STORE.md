# Microsoft Store Submission Guide for Headroom

This guide contains the exact metadata, questionnaire answers, and technical instructions required to submit Headroom to the **Microsoft Store** via **Microsoft Partner Center**.

---

## 1. Prerequisites

1. **Partner Center Account**:
   - Register at [partner.microsoft.com/dashboard](https://partner.microsoft.com/dashboard).
   - One-time registration fee: $19 USD (Individual account).

2. **Public Assets Ready**:
   - Privacy Policy URL: `https://github.com/redwan2003-bot/HeadRoom/blob/main/PRIVACY.md`
   - Source Code / Support URL: `https://github.com/redwan2003-bot/HeadRoom`
   - Release Asset URL: `https://github.com/redwan2003-bot/HeadRoom/releases/download/v1.1.0/Headroom-Setup-v1.1.0.exe` (or zip)

---

## 2. Store Submission Fields (Copy-Paste Ready)

### Product Management
- **Product Name**: `Headroom` (or `Headroom Memory Diagnostic`)
- **Category**: `Utilities & tools` > `File & system management`

### Pricing and Availability
- **Base Price**: `Free`
- **Markets**: All markets
- **Discoverability**: Public

### Age Ratings
- Take the standard IARC Questionnaire:
  - Violence: **No**
  - Fear: **No**
  - Sexuality: **No**
  - Gambling: **No**
  - Language: **No**
  - Controlled substances: **No**
  - Telemetry / Location sharing: **No**
  - User interaction / chat: **No**
- Expected Rating: **PEGI 3 / ESRB Everyone / General**.

### Store Listing Details
- **App Title**: `Headroom — Memory Diagnostic & Optimizer`
- **Short Description**:
  ```text
  Evidence-based native Win32 memory diagnostic and management utility for low-RAM PCs and gamers.
  ```
- **Description**:
  ```markdown
  Headroom is a lightweight, zero-telemetry native Windows memory utility engineered for transparency and evidence.

  Unlike traditional RAM cleaners that cosmetically flush standby cache at the expense of disk thrashing, Headroom distinguishes between true memory pressure and healthy Windows caching.

  Key Capabilities:
  • Real-Time Memory Telemetry: Live breakdown of In-Use, Standby cache priorities, Modified page list, Commit limit, and hard page faults.
  • Mini GDI Sparkline: 10-minute rolling RAM usage history with peak metrics.
  • Game & App Protection: Automatically suppresses actions when fullscreen games or foreground applications are running.
  • Interactive Exclusions: Protect your favorite games and critical software with custom exclusion lists.
  • Auditable Receipts: Logs memory freed and verifies 60-second subsequent disk read-back.
  • Ultra-Lightweight: Idles at 0.00% CPU and ~1.3 MB RAM.
  ```

- **Features**:
  - Live Win32 memory breakdown (In-Use, Standby, Commit, Page Faults)
  - 10-minute rolling GDI sparkline trend
  - Global hotkey daemon (Ctrl + Shift + H)
  - Interactive process exclusion management
  - Zero telemetry — 100% local operation

- **Search Terms / Keywords**:
  `memory`, `ram`, `diagnostic`, `optimizer`, `system monitor`, `win32`, `low ram`, `cache`

- **Privacy Policy URL**:
  `https://github.com/redwan2003-bot/HeadRoom/blob/main/PRIVACY.md`

- **Support Contact Info**:
  `https://github.com/redwan2003-bot/HeadRoom/issues`

---

## 3. Package Options in Partner Center

### Option A: Standard Win32 Installer (Recommended)
1. In Partner Center, select **Packages** > **Desktop application (installer)**.
2. Provide your GitHub Release installer URL:
   `https://github.com/redwan2003-bot/HeadRoom/releases/download/v1.1.0/Headroom-Setup-v1.1.0.exe`
3. Silent installer parameters: `/VERYSILENT /NORESTART`
4. The Store will test-download and verify the installer.

### Option B: MSIX Package
If you prefer Microsoft to host the binaries in an MSIX container:
1. Use Windows SDK `MakeAppx.exe` or the MSIX Packaging Tool.
2. Sign with a temporary certificate or your SignPath Foundation certificate.
3. Upload the `.msix` / `.msixupload` bundle directly to Partner Center.

---

## 4. Certification Notes for Reviewers

When submitting, add the following to **Notes for certification**:
```text
Headroom is an open-source, evidence-based memory utility written in Rust with native Win32 GDI.
It contains zero telemetry, requires no internet connection, and complies with Store Policies 10.2 and 10.4.
Source code: https://github.com/redwan2003-bot/HeadRoom
Privacy policy: https://github.com/redwan2003-bot/HeadRoom/blob/main/PRIVACY.md
```
