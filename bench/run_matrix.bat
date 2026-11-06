@echo off
echo =================================================================
echo  Headroom Benchmark Matrix Runner (VM Isolation Mode)
echo =================================================================
echo  NOTICE: Run this script inside a clean Virtual Machine (4 GB or 8 GB).
echo.

if not exist "bench\results" mkdir bench\results

echo [*] Starting Headroom Broker service in background...
start /B target\release\headroom-broker.exe --console > bench\results\broker.log 2>&1
timeout /t 2 /nobreak > nul

echo.
echo [1/4] Running Baseline Workload A (Plain Windows)...
powershell.exe -ExecutionPolicy Bypass -File bench\workload_relaunch.ps1 -Candidate baseline -Iterations 10
echo.

echo [2/4] Running Headroom Automatic Workload A (Trim Background Apps)...
powershell.exe -ExecutionPolicy Bypass -File bench\workload_relaunch.ps1 -Candidate headroom_auto -Iterations 10
echo.

echo [3/4] Running Headroom Stutter Profile Workload A (Low Standby Purge)...
powershell.exe -ExecutionPolicy Bypass -File bench\workload_relaunch.ps1 -Candidate headroom_stutter -Iterations 10
echo.

echo [4/4] Running Standby Cache Thrash Workload B...
powershell.exe -ExecutionPolicy Bypass -File bench\workload_cache.ps1 -Candidate baseline -Iterations 5
powershell.exe -ExecutionPolicy Bypass -File bench\workload_cache.ps1 -Candidate headroom_stutter -Iterations 5
powershell.exe -ExecutionPolicy Bypass -File bench\workload_cache.ps1 -Candidate headroom_full_purge -Iterations 5
echo.

echo [*] Stopping Broker process...
taskkill /F /IM headroom-broker.exe > nul 2>&1

echo =================================================================
echo  Benchmark complete. Raw results recorded in bench\results\
echo =================================================================
pause
