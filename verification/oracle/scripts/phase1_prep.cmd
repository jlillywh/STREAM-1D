@echo off
REM Phase 1 prep — uses pushd to map UNC paths; prefers Python (no PS policy / quoting issues).
REM Usage:
REM   verification\oracle\scripts\phase1_prep.cmd
REM   verification\oracle\scripts\phase1_prep.cmd -OpenRas
REM From UNC PowerShell cwd (recommended):
REM   py -3 verification\oracle\scripts\phase1_prep.py --open-ras

setlocal
pushd "%~dp0..\..\..\"
where py >nul 2>&1
if %ERRORLEVEL% equ 0 (
  py -3 verification\oracle\scripts\phase1_prep.py %*
  goto :finish
)
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0phase1_prep.ps1" %*
:finish
set RC=%ERRORLEVEL%
popd
exit /b %RC%
