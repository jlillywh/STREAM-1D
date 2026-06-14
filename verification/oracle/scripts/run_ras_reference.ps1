# Refresh HEC-RAS reference for a linked scenario (Windows — recommended).
# If execution policy blocks this script, use run_ras_reference.cmd instead.
# The repo .venv is usually WSL/Linux-only; this script uses Windows py/python directly.
# Usage:
#   .\verification\oracle\scripts\run_ras_reference.ps1
#   .\verification\oracle\scripts\run_ras_reference.ps1 -Scenario verification\oracle\scenarios\reach_mild_unsteady_linked.json
#   .\verification\oracle\scripts\run_ras_reference.ps1 -SkipRasRun -Verify

param(
    [string]$Scenario = "verification\oracle\scenarios\reach_mild_unsteady_linked.json",
    [switch]$SkipRasRun,
    [switch]$Verify,
    [switch]$NoVerify,
    [string]$RasExe = "C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
Set-Location $Root

$env:HECRAS_RAS_EXE = $RasExe

$argsList = @(
    "verification\oracle\scripts\run_ras_reference.py",
    "--scenario", $Scenario
)
if ($SkipRasRun) { $argsList += "--skip-ras-run" }
if ($Verify) { $argsList += "--verify" }
if ($NoVerify) { $argsList += "--no-verify" }

$py = $null
if (Get-Command py -ErrorAction SilentlyContinue) { $py = "py", "-3" }
elseif (Get-Command python -ErrorAction SilentlyContinue) { $py = "python" }
else { throw "Windows Python not found. Install Python 3 and: pip install -r verification\requirements-oracle-hecras.txt" }

& $py @argsList
exit $LASTEXITCODE
