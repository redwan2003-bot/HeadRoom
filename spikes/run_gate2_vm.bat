@echo off
echo =================================================================
echo Headroom Gate 2 VM Test Runner (Broker + CLI IPC Verification)
echo =================================================================
echo NOTICE: Run this script with Administrator privileges INSIDE A VM.
echo.

if not exist "results" mkdir results

echo [1/4] Starting Headroom Broker in test console mode...
start /B headroom-broker.exe --console > results\broker_vm.log 2>&1
timeout /t 2 /nobreak > nul

echo [2/4] Testing CLI unelevated/privileged action: standby-low...
headroom-cli.exe clean standby-low --json > results\gate2_standby_low.json
type results\gate2_standby_low.json
echo.

echo [3/4] Waiting 16s for broker rate-limit window...
timeout /t 16 /nobreak > nul

echo [4/4] Testing CLI privileged action: standby-all...
headroom-cli.exe clean standby-all --json > results\gate2_standby_all.json
type results\gate2_standby_all.json
echo.

echo =================================================================
echo Stopping broker background process...
taskkill /F /IM headroom-broker.exe > nul 2>&1

echo.
echo Gate 2 test suite finished. Verify results in:
echo   spikes\results\gate2_standby_low.json
echo   spikes\results\gate2_standby_all.json
echo   spikes\results\broker_vm.log
echo   %%LOCALAPPDATA%%\Headroom\receipts.jsonl
echo =================================================================
pause
