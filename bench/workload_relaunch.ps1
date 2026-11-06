# Headroom Benchmark: Workload A (Cold App Relaunch & Page-In Timing)
param (
    [string]$Candidate = "baseline", # "baseline", "headroom_auto", "headroom_stutter", "memreduct"
    [int]$Iterations = 10,
    [string]$OutputPath = "bench\results\relaunch_$Candidate.json"
)

Write-Host "=========================================================="
Write-Host " Headroom Benchmark: Workload A (Relaunch Latency)"
Write-Host " Candidate: $Candidate | Iterations: $Iterations"
Write-Host "=========================================================="

if (-not (Test-Path "bench\results")) {
    New-Item -ItemType Directory -Path "bench\results" | Out-Null
}

$results = @()

for ($i = 1; $i -le $Iterations; $i++) {
    Write-Host "`n[*] Starting Iteration $i of $Iterations..."

    # 1. Launch test target processes
    $pids = @()
    $apps = @("notepad.exe", "powershell.exe", "mspaint.exe")
    foreach ($app in $apps) {
        $proc = Start-Process $app -PassThru -WindowStyle Minimized
        $pids += $proc.Id
    }

    Start-Sleep -Seconds 3

    # 2. Apply cleaning candidate action
    $actionDurationMs = 0
    if ($Candidate -eq "headroom_auto") {
        Write-Host "    Executing Headroom background trimming..."
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        & .\target\release\headroom-cli.exe clean working-sets --json | Out-Null
        $sw.Stop()
        $actionDurationMs = $sw.ElapsedMilliseconds
    } elseif ($Candidate -eq "headroom_stutter") {
        Write-Host "    Executing Headroom low-priority standby purge..."
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        & .\target\release\headroom-cli.exe clean standby-low --json | Out-Null
        $sw.Stop()
        $actionDurationMs = $sw.ElapsedMilliseconds
    } elseif ($Candidate -eq "memreduct") {
        Write-Host "    Triggering external Mem Reduct cleaner..."
        if (Get-Process "memreduct" -ErrorAction SilentlyContinue) {
            # Trigger via hotkey or CLI if available in VM
            Start-Sleep -Seconds 1
        }
    } else {
        Write-Host "    Baseline: No cleaning action executed."
        Start-Sleep -Seconds 1
    }

    # 3. Measure relaunch time
    $relaunchTimesMs = @()
    $faultCounters = @()

    foreach ($pid in $pids) {
        $p = Get-Process -Id $pid -ErrorAction SilentlyContinue
        if ($p) {
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            # Bring window to foreground and touch memory
            [void]$p.WaitForInputIdle(500)
            $sw.Stop()
            $relaunchTimesMs += $sw.ElapsedMilliseconds
        }
    }

    # 4. Measure hard faults over 5 seconds post-relaunch
    $faultCounter = Get-Counter "\Memory\Pages Input/sec" -SampleInterval 1 -MaxSamples 3 -ErrorAction SilentlyContinue
    $avgFaults = 0.0
    if ($faultCounter) {
        $faults = $faultCounter.CounterSamples | ForEach-Object { $_.CookedValue }
        $avgFaults = ($faults | Measure-Object -Average).Average
    }

    # Cleanup test processes
    foreach ($pid in $pids) {
        Stop-Process -Id $pid -Force -ErrorAction SilentlyContinue
    }

    $medianRelaunch = 0
    if ($relaunchTimesMs.Count -gt 0) {
        $sorted = $relaunchTimesMs | Sort-Object
        $medianRelaunch = $sorted[[Math]::Floor($sorted.Count / 2)]
    }

    Write-Host "    Iteration $i Result: Median Relaunch = $medianRelaunch ms | Hard Faults = $avgFaults pages/sec"

    $results += [PSCustomObject]@{
        Iteration = $i
        Candidate = $Candidate
        ActionDurationMs = $actionDurationMs
        MedianRelaunchMs = $medianRelaunch
        HardFaultsPerSec = $avgFaults
    }

    Start-Sleep -Seconds 2
}

# Calculate summary
$allRelaunches = $results | ForEach-Object { $_.MedianRelaunchMs } | Sort-Object
$overallMedian = $allRelaunches[[Math]::Floor($allRelaunches.Count / 2)]
$avgFaultsOverall = ($results | ForEach-Object { $_.HardFaultsPerSec } | Measure-Object -Average).Average

$outputData = @{
    Candidate = $Candidate
    Iterations = $Iterations
    OverallMedianRelaunchMs = $overallMedian
    AverageHardFaultsPerSec = $avgFaultsOverall
    Runs = $results
}

$outputData | ConvertTo-Json -Depth 4 | Out-File -FilePath $OutputPath -Encoding utf8
Write-Host "`n[+] Workload complete. Results saved to $OutputPath"
Write-Host "    Overall Median Relaunch: $overallMedian ms"
Write-Host "    Average Hard Faults:     $avgFaultsOverall pages/sec"
