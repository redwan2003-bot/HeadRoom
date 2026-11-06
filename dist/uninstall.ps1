# Headroom v1.0 Clean Uninstaller
# Removes all binaries, shortcuts, and directories cleanly.

param (
    [switch]$PurgeUserData = $false
)

$ErrorActionPreference = "SilentlyContinue"

Write-Host "=========================================================="
Write-Host " Headroom v1.0 Clean Uninstaller"
Write-Host "=========================================================="

# 1. Terminate all processes
Write-Host "[*] Stopping Headroom processes..."
Get-Process -Name "headroom-agent", "headroom-dashboard", "headroom-cli", "headroom-broker" | Stop-Process -Force
Start-Sleep -Seconds 1

# 2. Remove Start Menu shortcuts
$startMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Headroom"
if (Test-Path $startMenuDir) {
    Remove-Item -Path $startMenuDir -Recurse -Force
    Write-Host "[+] Removed Start Menu shortcuts."
}

# 3. Remove Startup shortcut if exists
$startupShortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Startup\Headroom.lnk"
if (Test-Path $startupShortcut) {
    Remove-Item -Path $startupShortcut -Force
    Write-Host "[+] Removed Startup shortcut."
}

# 4. Remove installation directory
$installDir = Join-Path $env:LOCALAPPDATA "Programs\Headroom"
if (Test-Path $installDir) {
    Remove-Item -Path $installDir -Recurse -Force
    Write-Host "[+] Removed installation directory: $installDir"
}

# 5. Optionally remove user receipts and config
if ($PurgeUserData) {
    $userDataDir = Join-Path $env:LOCALAPPDATA "Headroom"
    if (Test-Path $userDataDir) {
        Remove-Item -Path $userDataDir -Recurse -Force
        Write-Host "[+] Removed user data and receipts directory: $userDataDir"
    }
}

Write-Host "=========================================================="
Write-Host " [SUCCESS] Headroom completely uninstalled. Nothing left behind."
Write-Host "=========================================================="
