@echo off
set "RAS_EXE=C:/Program Files (x86)/HEC/HEC-RAS/7.0.1/Ras.exe"
if not exist "C:/Users/jason/AppData/Local/stream1d_oracle/reach_mild/reach_mild.prj" (echo MISSING PRJ: C:/Users/jason/AppData/Local/stream1d_oracle/reach_mild/reach_mild.prj& exit /b 1)
if not exist "C:/Users/jason/AppData/Local/stream1d_oracle/reach_mild/reach_mild.p02" (echo MISSING PLAN: C:/Users/jason/AppData/Local/stream1d_oracle/reach_mild/reach_mild.p02& exit /b 1)
"%RAS_EXE%" -c "C:/Users/jason/AppData/Local/stream1d_oracle/reach_mild/reach_mild.prj" "C:/Users/jason/AppData/Local/stream1d_oracle/reach_mild/reach_mild.p02"
exit /b %ERRORLEVEL%
