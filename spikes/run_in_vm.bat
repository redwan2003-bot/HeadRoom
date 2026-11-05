@echo off
echo =================================================================
echo Headroom VM Test Runner (Spike 2 & Spike 3)
echo =================================================================
echo NOTICE: Run this script with Administrator privileges INSIDE A VM.
echo.

if not exist "results" mkdir results

echo [1/2] Running Spike 2 (Privileged Cleaning Actions Probe)...
spike2_priv_actions.exe
if %ERRORLEVEL% NEQ 0 (
    echo [-] Spike 2 failed with exit code %ERRORLEVEL%
) else (
    echo [+] Spike 2 complete. Output: results\spike2.json
)
echo.

echo [2/2] Running Spike 3 (Trimming vs Commit Invariant Probe)...
spike3_commit_invariant.exe
if %ERRORLEVEL% NEQ 0 (
    echo [-] Spike 3 failed with exit code %ERRORLEVEL%
) else (
    echo [+] Spike 3 complete. Output: results\spike3.json
)
echo.

echo =================================================================
echo All VM tests finished. Copy the 'results' folder back to 'spikes/results/'
echo on the development workstation.
echo =================================================================
pause
