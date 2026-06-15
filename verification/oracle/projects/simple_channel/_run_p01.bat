@echo off
set "RAS_EXE=C:/Program Files (x86)/HEC/HEC-RAS/7.0.1/Ras.exe"
if not exist "C:/Users/jason/AppData/Local/stream1d_oracle/simple_channel/simple_channel.prj" (echo MISSING PRJ: C:/Users/jason/AppData/Local/stream1d_oracle/simple_channel/simple_channel.prj& exit /b 1)
if not exist "C:/Users/jason/AppData/Local/stream1d_oracle/simple_channel/simple_channel.p01" (echo MISSING PLAN: C:/Users/jason/AppData/Local/stream1d_oracle/simple_channel/simple_channel.p01& exit /b 1)
"%RAS_EXE%" -c "C:/Users/jason/AppData/Local/stream1d_oracle/simple_channel/simple_channel.prj" "C:/Users/jason/AppData/Local/stream1d_oracle/simple_channel/simple_channel.p01"
exit /b %ERRORLEVEL%
