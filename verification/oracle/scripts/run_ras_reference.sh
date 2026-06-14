#!/usr/bin/env bash
# Refresh HEC-RAS reference from WSL (stages project to Windows LOCALAPPDATA).
#
# For fewer path issues, prefer Windows PowerShell instead:
#   .\verification\oracle\scripts\run_ras_reference.ps1
#
# Usage:
#   bash verification/oracle/scripts/run_ras_reference.sh
#   bash verification/oracle/scripts/run_ras_reference.sh --skip-ras-run --verify

set -eu
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT"

if [[ -f "$ROOT/.venv/bin/activate" ]]; then
  # shellcheck disable=SC1091
  source "$ROOT/.venv/bin/activate"
fi

export HECRAS_RAS_EXE="${HECRAS_RAS_EXE:-/mnt/c/Program Files (x86)/HEC/HEC-RAS/7.0.1/Ras.exe}"

SCENARIO="${SCENARIO:-verification/oracle/scenarios/reach_mild_unsteady_linked.json}"

exec python3 verification/oracle/scripts/run_ras_reference.py \
  --scenario "$SCENARIO" \
  "$@"
