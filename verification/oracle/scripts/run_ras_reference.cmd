@echo off
REM Refresh HEC-RAS reference (Windows). No PowerShell execution policy required.
REM Usage:
REM   verification\oracle\scripts\run_ras_reference.cmd
REM   verification\oracle\scripts\run_ras_reference.cmd --skip-ras-run --verify

setlocal
cd /d "%~dp0..\..\..\"
if not defined HECRAS_RAS_EXE set "HECRAS_RAS_EXE=C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe"

where py >nul 2>&1 && set "PY=py -3" || set "PY=python"
%PY% verification\oracle\scripts\run_ras_reference.py --scenario verification\oracle\scenarios\reach_mild_unsteady_linked.json %*
exit /b %ERRORLEVEL%
