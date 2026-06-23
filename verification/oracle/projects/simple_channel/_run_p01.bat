@echo off
set "RAS_EXE=C:/Program Files (x86)/HEC/HEC-RAS/7.0.1/Ras.exe"
set "STAGE=%LOCALAPPDATA%\stream1d_oracle\simple_channel"
if not exist "%STAGE%\simple_channel.prj" (echo MISSING PRJ: %STAGE%\simple_channel.prj& exit /b 1)
if not exist "%STAGE%\simple_channel.p01" (echo MISSING PLAN: %STAGE%\simple_channel.p01& exit /b 1)
"%RAS_EXE%" -c "%STAGE%\simple_channel.prj" "%STAGE%\simple_channel.p01"
exit /b %ERRORLEVEL%
