# Headroom v1.0 User-Level Clean Installer
# Installs to %LOCALAPPDATA%\Programs\Headroom without requiring administrator rights.

$ErrorActionPreference = "Stop"

$installDir = Join-Path $env:LOCALAPPDATA "Programs\Headroom"
$sourceDir = if (Test-Path "target\release") { "target\release" } else { "." }

Write-Host "=========================================================="
Write-Host " Headroom v1.0 Clean Installer"
Write-Host " Target Directory: $installDir"
Write-Host "=========================================================="

# 1. Stop existing running instances
Write-Host "[*] Stopping running Headroom processes..."
Get-Process -Name "headroom-agent", "headroom-dashboard", "headroom-cli" -ErrorAction SilentlyContinue | Stop-Process -Force

# 2. Create destination directory
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

# 3. Copy binaries
$binaries = @("headroom-agent.exe", "headroom-dashboard.exe", "headroom-cli.exe")
foreach ($bin in $binaries) {
    $src = Join-Path $sourceDir $bin
    if (Test-Path $src) {
        Copy-Item -Path $src -Destination (Join-Path $installDir $bin) -Force
        Write-Host "    [+] Deployed: $bin"
    } else {
        Write-Warning "[-] Source binary not found: $src"
    }
}

# 4. Create Start Menu Shortcut
$startMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Headroom"
if (-not (Test-Path $startMenuDir)) {
    New-Item -ItemType Directory -Path $startMenuDir -Force | Out-Null
}

$wshShell = New-Object -ComObject WScript.Shell

$shortcutPath = Join-Path $startMenuDir "Headroom.lnk"
$shortcut = $wshShell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = Join-Path $installDir "headroom-agent.exe"
$shortcut.WorkingDirectory = $installDir
$shortcut.Description = "Headroom Evidence-Based Memory Diagnostic Utility"
$shortcut.Save()

$dashShortcutPath = Join-Path $startMenuDir "Headroom Diagnostic Dashboard.lnk"
$dashShortcut = $wshShell.CreateShortcut($dashShortcutPath)
$dashShortcut.TargetPath = Join-Path $installDir "headroom-dashboard.exe"
$dashShortcut.WorkingDirectory = $installDir
$dashShortcut.Description = "Headroom Diagnostic Dashboard"
$dashShortcut.Save()

# 5. Launch agent
Start-Process (Join-Path $installDir "headroom-agent.exe")

Write-Host "=========================================================="
Write-Host " [SUCCESS] Headroom installed and running in the system tray."
Write-Host " To uninstall, run dist\uninstall.ps1."
Write-Host "=========================================================="
