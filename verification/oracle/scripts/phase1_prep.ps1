# Phase 1 — GUI reference truth (reach_mild)
#
# Run from Windows PowerShell at repo root. No headless RAS in this phase.
#
#   .\verification\oracle\scripts\phase1_prep.ps1
#   .\verification\oracle\scripts\phase1_prep.ps1 -OpenRas
#
# After GUI compute completes:
#   .\verification\oracle\scripts\phase1_capture_after_gui.ps1

param(
    [switch]$OpenRas,
    [string]$RasExe = "C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
$Source = Join-Path $Root "verification\oracle\projects\reach_mild"
$Stage = Join-Path $env:USERPROFILE "Documents\hecras_testing\reach_mild"
$Prj = Join-Path $Stage "reach_mild.prj"

Write-Host "=== Phase 1 prep — reach_mild GUI session ===" -ForegroundColor Cyan
Write-Host ""

Write-Host "1.1 Kill stray HEC-RAS processes..."
Get-Process Ras*, PipeServer -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1
Write-Host "    Done."
Write-Host ""

if (-not (Test-Path (Join-Path $Source "reach_mild.prj"))) {
    throw "Source project not found: $Source"
}

Write-Host "1.2 Stage fresh project copy (native Windows path)..."
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
$keep = @(
    "reach_mild.prj", "reach_mild.g01", "reach_mild.u02", "reach_mild.p02",
    "reference_wsel_reach_mild_unsteady.json", "README.md"
)
Get-ChildItem $Stage -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -notin $keep -and $_.Extension -ne ".hdf" } |
    Remove-Item -Force -ErrorAction SilentlyContinue
foreach ($name in $keep) {
    $src = Join-Path $Source $name
    if (Test-Path $src) {
        Copy-Item -Force $src (Join-Path $Stage $name)
    }
}
Write-Host "    Source: $Source"
Write-Host "    Stage:  $Stage"
Write-Host ""

Write-Host "Open this project in HEC-RAS 7.0.1:" -ForegroundColor Yellow
Write-Host "  $Prj"
Write-Host ""
Write-Host "Phase 1 GUI checklist:"
Write-Host "  1.3 Unsteady Flow Editor -> verify BCs -> Save reach_mild.u02 (if prompted)"
Write-Host "  1.4 Run -> Compute Plan 02 (unsteady)"
Write-Host "  1.5 Record terminal WSEL at RM 20.208, 20.189, 20.095"
Write-Host ""
Write-Host "Expected HDF after compute:"
Write-Host "  $Stage\reach_mild.p02.hdf"
Write-Host ""
Write-Host "When compute finishes, run:"
Write-Host "  .\verification\oracle\scripts\phase1_capture_after_gui.ps1" -ForegroundColor Green
Write-Host ""

if ($OpenRas) {
    if (-not (Test-Path $RasExe)) {
        throw "Ras.exe not found: $RasExe"
    }
    Write-Host "Launching HEC-RAS (open the project manually — .prj is not file-associated)..."
    Start-Process -FilePath $RasExe
    Write-Host "Then: File > Open Project >"
    Write-Host "  $Prj"
}
