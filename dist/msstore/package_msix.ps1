# Headroom Microsoft Store & MSIX Packager
param(
    [string]$Version = "1.1.0",
    [string]$Publisher = "CN=HeadroomContributors"
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$ReleaseDir = Join-Path $RepoRoot "target\release"
$LayoutDir = Join-Path $RepoRoot "target\msix-layout"
$OutputMsix = Join-Path $ReleaseDir "Headroom-v$Version.msix"

Write-Host "=== Headroom MSIX / Microsoft Store Packager ===" -ForegroundColor Cyan
Write-Host "Target Version: $Version"

# 1. Verify release binaries exist
$requiredBinaries = @("headroom-dashboard.exe", "headroom-agent.exe", "headroom-cli.exe")
foreach ($bin in $requiredBinaries) {
    $binPath = Join-Path $ReleaseDir $bin
    if (-not (Test-Path $binPath)) {
        throw "Missing required release binary: $binPath. Run 'cargo build --release' first."
    }
}

# 2. Re-create layout directory
if (Test-Path $LayoutDir) {
    Remove-Item -Recurse -Force $LayoutDir
}
New-Item -ItemType Directory -Path $LayoutDir -Force | Out-Null
New-Item -ItemType Directory -Path "$LayoutDir\Assets" -Force | Out-Null

# 3. Copy binaries and documentation
Write-Host "Staging application binaries..."
foreach ($bin in $requiredBinaries) {
    Copy-Item (Join-Path $ReleaseDir $bin) $LayoutDir -Force
}
Copy-Item (Join-Path $RepoRoot "LICENSE") $LayoutDir -Force
Copy-Item (Join-Path $RepoRoot "README.md") $LayoutDir -Force

# 4. Copy Visual Assets
Write-Host "Copying store visual assets..."
$assetsSrc = Join-Path $PSScriptRoot "Assets"
if (-not (Test-Path $assetsSrc)) {
    & "$PSScriptRoot\generate_icons.ps1"
}
Copy-Item "$assetsSrc\*" "$LayoutDir\Assets\" -Force

# 5. Copy and configure AppxManifest.xml
Write-Host "Configuring AppxManifest.xml..."
$manifestContent = Get-Content (Join-Path $PSScriptRoot "AppxManifest.xml") -Raw
$manifestContent = $manifestContent -replace 'Publisher="CN=[^"]+"', "Publisher=`"$Publisher`""
$manifestContent = $manifestContent -replace 'Version="[^"]+"', "Version=`"$Version.0`""
Set-Content -Path (Join-Path $LayoutDir "AppxManifest.xml") -Value $manifestContent -Encoding UTF8

# 6. Locate MakeAppx.exe
Write-Host "Searching for Windows SDK MakeAppx.exe..."
$makeAppx = Get-Command makeappx.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
if (-not $makeAppx) {
    $sdkPaths = @(
        "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\makeappx.exe",
        "C:\Program Files\Windows Kits\10\bin\*\x64\makeappx.exe"
    )
    $found = Get-Item $sdkPaths -ErrorAction SilentlyContinue | Select-Object -Last 1
    if ($found) {
        $makeAppx = $found.FullName
    }
}

if ($makeAppx) {
    Write-Host "Found MakeAppx: $makeAppx" -ForegroundColor Green
    Write-Host "Creating MSIX package: $OutputMsix"
    & $makeAppx pack /d $LayoutDir /p $OutputMsix /nv /o
    
    $hash = (Get-FileHash $OutputMsix -Algorithm SHA256).Hash
    Write-Host "MSIX Package Created Successfully!" -ForegroundColor Green
    Write-Host "Path:   $OutputMsix"
    Write-Host "SHA256: $hash"
} else {
    Write-Host "Notice: Windows SDK MakeAppx.exe was not detected on this machine." -ForegroundColor Yellow
    Write-Host "Staged complete MSIX layout at: $LayoutDir" -ForegroundColor Green
    Write-Host "When building on CI (windows-latest) or a machine with Windows SDK installed,"
    Write-Host "run: makeappx pack /d `"$LayoutDir`" /p `"$OutputMsix`" /nv"
}

Write-Host "MSIX staging complete." -ForegroundColor Cyan
