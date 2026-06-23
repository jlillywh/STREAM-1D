@echo off
setlocal
pushd "%~dp0..\..\..\"
if not defined HECRAS_RAS_EXE set "HECRAS_RAS_EXE=C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe"
where py >nul 2>&1
if %ERRORLEVEL% equ 0 (
  py -3 verification\oracle\scripts\phase1_capture_after_gui.py %*
  goto :finish
)
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0phase1_capture_after_gui.ps1" %*
:finish
set RC=%ERRORLEVEL%
popd
exit /b %RC%
