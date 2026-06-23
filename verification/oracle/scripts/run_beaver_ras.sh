#!/usr/bin/env bash
# Headless HEC-RAS plan 03 for Beaver Creek + optional STREAM-1D compare.
#
# Prerequisites (once):
#   python3 -m venv .venv
#   source .venv/bin/activate
#   pip install -r verification/requirements-oracle-hecras.txt
#   maturin develop --features python   # for --verify
#
# Environment:
#   HECRAS_RAS_EXE   Path to Ras.exe (auto-detected on WSL if under /mnt/c/...)
#   HECRAS_VERSION   e.g. 7.0.1 (optional)
#
# Examples:
#   # Full pipeline: RAS run → HDF extract → reference JSON → STREAM-1D compare
#   bash verification/oracle/scripts/run_beaver_ras.sh
#
#   # HEC-RAS only (no STREAM-1D verify)
#   bash verification/oracle/scripts/run_beaver_ras.sh --no-verify
#
#   # Compare STREAM-1D to committed reference (no HEC-RAS)
#   bash verification/oracle/scripts/run_beaver_ras.sh --skip-ras-run --verify
#
#   # Re-extract from an existing plan HDF
#   bash verification/oracle/scripts/run_beaver_ras.sh --skip-ras-run \
#     --hdf verification/oracle/projects/beaver/beaver.p03.hdf
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

if [[ -f "$ROOT/.venv/bin/activate" ]]; then
  # shellcheck disable=SC1091
  source "$ROOT/.venv/bin/activate"
fi

PY="${PYTHON:-python3}"
if ! command -v "$PY" >/dev/null 2>&1; then
  echo "ERROR: python3 not found" >&2
  exit 2
fi

if [[ -z "${HECRAS_RAS_EXE:-}" && -f "/mnt/c/Program Files (x86)/HEC/HEC-RAS/7.0.1/Ras.exe" ]]; then
  export HECRAS_RAS_EXE="/mnt/c/Program Files (x86)/HEC/HEC-RAS/7.0.1/Ras.exe"
fi

# Kill stale HEC-RAS before re-run (WSL — note .exe suffix):
#   taskkill.exe /F /IM Ras.exe /IM RasPlotDriver.exe /IM PipeServer.exe
# Reset polluted stage folder if a prior GUI run corrupted beaver.prj:
#   rm -rf "/mnt/c/Users/jason/Documents/hecras_testing/beaver"
# WSL now uses Windows ras-commander by default (not Ras.exe -c).
# Legacy batch path: HECRAS_USE_RAS_EXE_BATCH=1 bash verification/oracle/scripts/run_beaver_ras.sh
export HECRAS_RUN_TIMEOUT_SEC="${HECRAS_RUN_TIMEOUT_SEC:-3600}"

SCENARIO="verification/oracle/scenarios/beaver_unsteady_linked.json"

exec "$PY" verification/oracle/scripts/run_ras_reference.py \
  --scenario "$SCENARIO" \
  "$@"
