# Phase 1 — capture GUI run into committed reference (after Plan 02 HDF exists)
#
#   .\verification\oracle\scripts\phase1_capture_after_gui.ps1
#   .\verification\oracle\scripts\phase1_capture_after_gui.ps1 -HdfPath C:\path\to\reach_mild.p02.hdf

param(
    [string]$HdfPath = "",
    [string]$StageDir = "",
    [string]$Scenario = "verification\oracle\scenarios\reach_mild_unsteady_linked.json",
    [string]$RasExe = "C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
Set-Location $Root

if (-not $StageDir) {
    $StageDir = Join-Path $env:USERPROFILE "Documents\hecras_testing\reach_mild"
}
if (-not $HdfPath) {
    $HdfPath = Join-Path $StageDir "reach_mild.p02.hdf"
}

$RepoProject = Join-Path $Root "verification\oracle\projects\reach_mild"
$RepoHdf = Join-Path $RepoProject "reach_mild.p02.hdf"

Write-Host "=== Phase 1 capture — HDF to reference JSON ===" -ForegroundColor Cyan
Write-Host "HDF: $HdfPath"

if (-not (Test-Path $HdfPath)) {
    Write-Host ""
    Write-Host "ERROR: HDF not found. Complete Phase 1.4 in HEC-RAS GUI first." -ForegroundColor Red
    Write-Host "  Expected: $HdfPath"
    Write-Host "  Or pass:  -HdfPath path\to\reach_mild.p02.hdf"
    exit 1
}

$env:HECRAS_RAS_EXE = $RasExe

$py = $null
if (Get-Command py -ErrorAction SilentlyContinue) { $py = @("py", "-3") }
elseif (Get-Command python -ErrorAction SilentlyContinue) { $py = @("python") }
else { throw "Windows Python not found" }

Write-Host ""
Write-Host "1.6 Extract terminal WSEL -> reference JSON..."
& $py @(
    "verification\oracle\scripts\run_ras_reference.py",
    "--scenario", $Scenario,
    "--skip-ras-run",
    "--hdf", $HdfPath,
    "--no-verify"
)
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Copy-Item -Force $HdfPath $RepoHdf
Write-Host "Copied HDF -> $RepoHdf"

$stageU02 = Join-Path $StageDir "reach_mild.u02"
$repoU02 = Join-Path $RepoProject "reach_mild.u02"
if ((Test-Path $stageU02) -and ($stageU02 -ne $repoU02)) {
    Copy-Item -Force $stageU02 $repoU02
    Write-Host "Copied GUI u02 -> $repoU02"
}

Write-Host ""
Write-Host "1.7 Re-run verify (WSL or Windows)..."
& $py @(
    "verification\oracle\scripts\run_ras_reference.py",
    "--scenario", $Scenario,
    "--skip-ras-run",
    "--verify"
)
exit $LASTEXITCODE
