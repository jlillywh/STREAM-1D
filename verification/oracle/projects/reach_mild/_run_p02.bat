@echo off
set "RAS_EXE=C:/Program Files (x86)/HEC/HEC-RAS/7.0.1/Ras.exe"
set "STAGE=%LOCALAPPDATA%\stream1d_oracle\reach_mild"
if not exist "%STAGE%\reach_mild.prj" (echo MISSING PRJ: %STAGE%\reach_mild.prj& exit /b 1)
if not exist "%STAGE%\reach_mild.p02" (echo MISSING PLAN: %STAGE%\reach_mild.p02& exit /b 1)
"%RAS_EXE%" -c "%STAGE%\reach_mild.prj" "%STAGE%\reach_mild.p02"
exit /b %ERRORLEVEL%
