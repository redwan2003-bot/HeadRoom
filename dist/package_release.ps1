# Headroom Release Packaging Script
$ErrorActionPreference = "Stop"

$root = Resolve-Path "$PSScriptRoot\.."
$targetDir = "$root\target\release"
$staging = "$targetDir\staging"
$zipPath = "$targetDir\Headroom-v1.1.0-windows-x64.zip"

Write-Host "Creating staging directory at $staging..."
if (Test-Path $staging) { Remove-Item -Recurse -Force $staging }
New-Item -ItemType Directory -Path $staging | Out-Null

Copy-Item "$targetDir\headroom-agent.exe" "$staging\"
Copy-Item "$targetDir\headroom-dashboard.exe" "$staging\"
Copy-Item "$targetDir\headroom-cli.exe" "$staging\"
Copy-Item "$targetDir\headroom-broker.exe" "$staging\"
Copy-Item "$root\dist\install.ps1" "$staging\"
Copy-Item "$root\dist\uninstall.ps1" "$staging\"
Copy-Item "$root\README.md" "$staging\"
Copy-Item "$root\LICENSE" "$staging\"
New-Item -ItemType File -Path "$staging\portable" | Out-Null

Write-Host "Compressing to $zipPath..."
if (Test-Path $zipPath) { Remove-Item -Force $zipPath }
Compress-Archive -Path "$staging\*" -DestinationPath $zipPath
Remove-Item -Recurse -Force $staging

$hash = (Get-FileHash $zipPath -Algorithm SHA256).Hash
Write-Host "Zip Archive SHA256: $hash"
Write-Host "Size: $((Get-Item $zipPath).Length / 1KB) KB"

$hash | Out-File -Encoding utf8 "$targetDir\ZIP_SHA256.txt"
