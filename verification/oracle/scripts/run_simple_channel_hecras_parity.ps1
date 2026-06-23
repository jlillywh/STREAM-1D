# Run simple channel HEC-RAS parity from Windows Python (recommended if WSL automation fails).
#
# Usage (PowerShell):
#   cd \\wsl.localhost\Ubuntu\home\jason\Lillywhite_Consulting\lillywhite_engine\STREAM-1D
#   .\.venv\Scripts\Activate.ps1
#   pip install ras-commander
#   .\verification\oracle\scripts\run_simple_channel_hecras_parity.ps1

param(
    [switch]$SkipRasRun
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
$Stage = Join-Path $env:LOCALAPPDATA "stream1d_oracle\simple_channel"
$Source = Join-Path $Root "verification\oracle\projects\simple_channel"

Write-Host "Staging project -> $Stage"
if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Get-ChildItem $Source -File | Where-Object { $_.Extension -notin ".hdf" } | Copy-Item -Destination $Stage

$RasExe = if ($env:HECRAS_RAS_EXE) { $env:HECRAS_RAS_EXE } else { "C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe" }

if (-not $SkipRasRun) {
    Write-Host "Running HEC-RAS plan 01 from $Stage ..."
    Push-Location $Stage
    try {
        & $RasExe -c "$Stage\simple_channel.prj" "$Stage\simple_channel.p01"
        if ($LASTEXITCODE -ne 0) { throw "Ras.exe exit code $LASTEXITCODE" }
    } finally {
        Pop-Location
    }
    if (Test-Path "$Stage\simple_channel.p01.hdf") {
        Copy-Item "$Stage\simple_channel.p01.hdf" (Join-Path $Source "simple_channel.p01.hdf") -Force
        Write-Host "Copied HDF to repo project folder."
    } else {
        Write-Warning "No simple_channel.p01.hdf in staged folder after run."
    }
}

Set-Location $Root
$pyArgs = @("verification\oracle\scripts\run_simple_channel_hecras_parity.py")
if ($SkipRasRun) { $pyArgs += "--skip-ras-run" }
python @pyArgs
