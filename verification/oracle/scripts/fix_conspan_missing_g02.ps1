# Fix ConSpan project missing ConSpan.g02
#
# HEC-RAS opens the geometry named in ConSpan.prj (often g02 after migration or
# an incomplete copy). This folder usually only has ConSpan.g01 from the legacy example.
#
# Usage (PowerShell, from the ConSpan project folder):
#   .\fix_conspan_missing_g02.ps1
# Or from repo:
#   powershell -File verification/oracle/scripts/fix_conspan_missing_g02.ps1 `
#     -ProjectDir "$env:USERPROFILE\Documents\hecras_testing\ConSpan"

param(
    [string]$ProjectDir = (Split-Path -Parent $MyInvocation.MyCommand.Path)
)

$ErrorActionPreference = "Stop"
if (-not (Test-Path $ProjectDir)) {
    Write-Error "Project directory not found: $ProjectDir"
}

$g01 = Get-ChildItem -Path $ProjectDir -Filter "*.g01" -File | Select-Object -First 1
$prj = Get-ChildItem -Path $ProjectDir -Filter "*.prj" -File | Select-Object -First 1

Write-Host "Project: $ProjectDir"
Get-ChildItem $ProjectDir -File | Select-Object Name, Length | Format-Table -AutoSize

if (-not $g01) {
    Write-Error "No .g01 geometry found. Copy ConSpan.g01 from verification/oracle/projects/conspan/ first."
}

$stem = $g01.BaseName -replace '\.g01$',''
$g02Path = Join-Path $ProjectDir ($stem + '.g02')

Write-Host "`nOption A: copy geometry g01 -> g02"
if (-not (Test-Path $g02Path)) {
    Copy-Item -LiteralPath $g01.FullName -Destination $g02Path
    Write-Host "Created $g02Path from $($g01.Name)"
} else {
    Write-Host "Already exists: $g02Path"
}

if ($prj) {
    Write-Host "`nOption B: patch $($prj.Name) Current Geom -> g01"
    $text = Get-Content -LiteralPath $prj.FullName -Raw -Encoding UTF8
    $patched = $text -replace 'Current Geom=g02','Current Geom=g01'
    if ($patched -ne $text) {
        Copy-Item -LiteralPath $prj.FullName -Destination ($prj.FullName + '.bak')
        Set-Content -LiteralPath $prj.FullName -Value $patched -Encoding UTF8 -NoNewline
        Write-Host "Patched Current Geom to g01 (backup: $($prj.Name).bak)"
    } else {
        Write-Host "No Current Geom=g02 in prj (may already be g01)."
    }
} else {
    Write-Host "`nNo .prj found — copy ConSpan.prj from verification/oracle/projects/conspan/"
}

Write-Host "`nNext: open the project in HEC-RAS 6.x. If prompted to migrate, accept."
Write-Host "Then add unsteady plan 02 using conspan.u02 from write_conspan_u02.py"
