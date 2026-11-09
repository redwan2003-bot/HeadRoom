# Microsoft Defender Security Intelligence (WDSI) Whitelisting Preparation
$ErrorActionPreference = 'Stop'

$root = Resolve-Path "$PSScriptRoot\.."
$targetDir = "$root\target\release"
$wdsiZip = "$targetDir\Headroom-WDSI-Submission-v1.1.0.zip"
$staging = "$targetDir\wdsi_staging"

Write-Host '=== Microsoft Defender Security Intelligence (WDSI) Packager ==='
Write-Host 'Packaging clean binaries for false-positive prevention and pre-whitelisting...'

if (Test-Path $staging) { Remove-Item -Recurse -Force $staging }
New-Item -ItemType Directory -Path $staging | Out-Null

Copy-Item "$targetDir\headroom-agent.exe" "$staging\"
Copy-Item "$targetDir\headroom-dashboard.exe" "$staging\"
Copy-Item "$targetDir\headroom-cli.exe" "$staging\"
Copy-Item "$targetDir\headroom-broker.exe" "$staging\"
Copy-Item "$root\README.md" "$staging\"
Copy-Item "$root\LICENSE" "$staging\"

if (Test-Path $wdsiZip) { Remove-Item -Force $wdsiZip }
Compress-Archive -Path "$staging\*" -DestinationPath $wdsiZip
Remove-Item -Recurse -Force $staging

$hash = (Get-FileHash $wdsiZip -Algorithm SHA256).Hash
Write-Host 'WDSI Submission Package Created:'
Write-Host "File: $wdsiZip"
Write-Host "SHA256: $hash"
Write-Host 'Submission Steps:'
Write-Host '1. Navigate to: https://www.microsoft.com/en-us/wdsi/filesubmission'
Write-Host '2. Select: Software developer -> I believe this file is clean'
Write-Host '3. Product Name: Headroom'
Write-Host "4. Upload: $wdsiZip"
Write-Host '5. Description: Headroom open-source native Win32 memory tool (MIT License, 0 telemetry)'
