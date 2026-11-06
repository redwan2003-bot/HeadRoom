# Headroom Benchmark: Workload B (Standby Cache Eviction & Read-Back Thrash)
param (
    [string]$Candidate = "baseline", # "baseline", "headroom_stutter", "headroom_full_purge"
    [int]$Iterations = 5,
    [string]$OutputPath = "bench\results\cache_$Candidate.json"
)

Write-Host "=========================================================="
Write-Host " Headroom Benchmark: Workload B (Cache Eviction Thrash)"
Write-Host " Candidate: $Candidate | Iterations: $Iterations"
Write-Host "=========================================================="

if (-not (Test-Path "bench\results")) {
    New-Item -ItemType Directory -Path "bench\results" | Out-Null
}

$tempDir = Join-Path $env:TEMP "HeadroomBenchCache"
if (-not (Test-Path $tempDir)) {
    New-Item -ItemType Directory -Path $tempDir | Out-Null
}

$testFile = Join-Path $tempDir "cache_test.bin"
$fileSizeMB = 512

# Create dummy test file if not exists
if (-not (Test-Path $testFile) -or (Get-Item $testFile).Length -lt ($fileSizeMB * 1024 * 1024)) {
    Write-Host "[*] Creating $fileSizeMB MB test payload file: $testFile..."
    $buffer = New-Object byte[] (1024 * 1024)
    (New-Object System.Random).NextBytes($buffer)
    $stream = [System.IO.File]::Create($testFile)
    for ($b = 0; $b -lt $fileSizeMB; $b++) {
        $stream.Write($buffer, 0, $buffer.Length)
    }
    $stream.Close()
}

$results = @()

for ($i = 1; $i -le $Iterations; $i++) {
    Write-Host "`n[*] Starting Iteration $i of $Iterations..."

    # 1. Warm cache: read file into memory/standby
    [System.IO.File]::ReadAllBytes($testFile) | Out-Null
    Start-Sleep -Seconds 1

    # 2. Trigger cleaning candidate
    $actionMs = 0
    if ($Candidate -eq "headroom_stutter") {
        Write-Host "    Executing Headroom low-priority standby purge..."
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        & .\target\release\headroom-cli.exe clean standby-low --json | Out-Null
        $sw.Stop()
        $actionMs = $sw.ElapsedMilliseconds
    } elseif ($Candidate -eq "headroom_full_purge") {
        Write-Host "    Executing Headroom full standby purge..."
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        & .\target\release\headroom-cli.exe clean standby-all --json | Out-Null
        $sw.Stop()
        $actionMs = $sw.ElapsedMilliseconds
    } else {
        Write-Host "    Baseline: No cleaning action."
        Start-Sleep -Seconds 1
    }

    # 3. Time re-reading the file back into memory
    $swRead = [System.Diagnostics.Stopwatch]::StartNew()
    $bytes = [System.IO.File]::ReadAllBytes($testFile)
    $swRead.Stop()
    $readDurationMs = $swRead.ElapsedMilliseconds
    $throughputMBs = if ($readDurationMs -gt 0) {
        [Math]::Round(($fileSizeMB / ($readDurationMs / 1000.0)), 2)
    } else {
        0.0
    }

    Write-Host "    Readback completed in $readDurationMs ms ($throughputMBs MB/s)"

    $results += [PSCustomObject]@{
        Iteration = $i
        Candidate = $Candidate
        ActionDurationMs = $actionMs
        ReadDurationMs = $readDurationMs
        ThroughputMBs = $throughputMBs
    }

    Start-Sleep -Seconds 2
}

# Cleanup
if (Test-Path $tempDir) {
    Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
}

$allDurations = $results | ForEach-Object { $_.ReadDurationMs } | Sort-Object
$medianDuration = $allDurations[[Math]::Floor($allDurations.Count / 2)]
$avgThroughput = ($results | ForEach-Object { $_.ThroughputMBs } | Measure-Object -Average).Average

$outputData = @{
    Candidate = $Candidate
    Iterations = $Iterations
    OverallMedianReadDurationMs = $medianDuration
    AverageThroughputMBs = $avgThroughput
    Runs = $results
}

$outputData | ConvertTo-Json -Depth 4 | Out-File -FilePath $OutputPath -Encoding utf8
Write-Host "`n[+] Cache test complete. Results saved to $OutputPath"
Write-Host "    Median Readback Duration: $medianDuration ms"
Write-Host "    Average Readback Speed:   $avgThroughput MB/s"
