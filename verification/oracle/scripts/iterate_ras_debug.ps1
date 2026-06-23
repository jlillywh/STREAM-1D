# Run HEC-RAS debug iterations; logs UTF-8 to verification/oracle/logs/ras_iterate_latest.log
# Usage (PowerShell):
#   .\verification\oracle\scripts\iterate_ras_debug.ps1
#   .\verification\oracle\scripts\iterate_ras_debug.ps1 -ConspanOnly
#   .\verification\oracle\scripts\iterate_ras_debug.ps1 -UseRasCommander

param(
    [switch]$ConspanOnly,
    [switch]$ReachMildOnly,
    [switch]$UseRasCommander,
    [switch]$SkipRasRun
)

$ErrorActionPreference = "Continue"
$Root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
$LogDir = Join-Path $Root "verification\oracle\logs"
$LogFile = Join-Path $LogDir "ras_iterate_latest.log"
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

function Write-Log($Text) {
    $Text | Out-File -FilePath $LogFile -Append -Encoding utf8
    Write-Host $Text
}

Remove-Item -Force $LogFile -ErrorAction SilentlyContinue
Write-Log "=== RAS iterate $(Get-Date -Format o) ==="
Write-Log "Root: $Root"

if (-not $env:HECRAS_RAS_EXE) {
    $env:HECRAS_RAS_EXE = 'C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe'
}
Write-Log "HECRAS_RAS_EXE=$($env:HECRAS_RAS_EXE)"

Get-Process Ras*, PipeServer -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Write-Log "Killed leftover Ras/PipeServer processes (if any)."

Set-Location $Root
if ($UseRasCommander) { $env:HECRAS_USE_RAS_COMMANDER = '1' } else { Remove-Item Env:HECRAS_USE_RAS_COMMANDER -ErrorAction SilentlyContinue }

function Invoke-Logged([string]$Label, [string[]]$Args) {
    Write-Log ""
    Write-Log "========== $Label =========="
    Write-Log ("py -3 " + ($Args -join ' '))
    & py -3 @Args 2>&1 | ForEach-Object { Write-Log $_ }
    Write-Log "EXIT=$LASTEXITCODE"
    return $LASTEXITCODE
}

function Show-Staging([string]$Name) {
    $stage = Join-Path $env:LOCALAPPDATA "stream1d_oracle\$Name"
    Write-Log ""
    Write-Log "--- staging: $stage ---"
    if (-not (Test-Path $stage)) {
        Write-Log "(missing)"
        return
    }
    Get-ChildItem $stage -File | ForEach-Object { Write-Log $_.Name }
    foreach ($pat in @('*.hdf', '*data_errors*', '*computeMsgs*', '_compute_p*.log')) {
        Get-ChildItem $stage -Filter $pat -ErrorAction SilentlyContinue | ForEach-Object {
            Write-Log "--- $($_.Name) ---"
            Get-Content $_.FullName -ErrorAction SilentlyContinue | Select-Object -Last 30 | ForEach-Object { Write-Log $_ }
        }
    }
}

if (-not $ReachMildOnly) {
    Remove-Item -Recurse -Force (Join-Path $env:LOCALAPPDATA "stream1d_oracle\conspan") -ErrorAction SilentlyContinue
    if ($SkipRasRun) {
        Invoke-Logged "conspan skip-ras-run" @('verification\oracle\scripts\run_ras_reference.py', '--scenario', 'verification\oracle\scenarios\reach_mild_unsteady_linked.json', '--skip-ras-run', '--no-verify') | Out-Null
    } else {
        Invoke-Logged "conspan smoke headless" @('verification\oracle\scripts\smoke_conspan_headless.py') | Out-Null
        Show-Staging "conspan"
    }
}

if (-not $ConspanOnly) {
    Remove-Item -Recurse -Force (Join-Path $env:LOCALAPPDATA "stream1d_oracle\reach_mild") -ErrorAction SilentlyContinue
    if ($SkipRasRun) {
        Invoke-Logged "reach_mild skip-ras-run" @('verification\oracle\scripts\run_ras_reference.py', '--scenario', 'verification\oracle\scenarios\reach_mild_unsteady_linked.json', '--skip-ras-run', '--no-verify') | Out-Null
    } else {
        Invoke-Logged "reach_mild headless" @('verification\oracle\scripts\run_ras_reference.py', '--scenario', 'verification\oracle\scenarios\reach_mild_unsteady_linked.json', '--no-verify') | Out-Null
        Show-Staging "reach_mild"
    }
}

Write-Log ""
Write-Log "=== done $(Get-Date -Format o) ==="
Write-Log "Log: $LogFile"
