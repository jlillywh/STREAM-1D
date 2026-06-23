#!/usr/bin/env bash
# Compare STREAM-1D Beaver Creek vs committed HEC-RAS reference (no live RAS).
#
# Use this for day-to-day parity work. Refresh the reference only when inputs
# change materially — run plan 03 once in the HEC-RAS GUI, then:
#   py -3 verification/oracle/scripts/extract_hdf_wsel.py \\
#     --hdf path/to/beaver.p03.hdf \\
#     --out verification/oracle/projects/beaver/reference_wsel_hdf_plan03.json
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"
if [[ -f "$ROOT/.venv/bin/activate" ]]; then
  # shellcheck disable=SC1091
  source "$ROOT/.venv/bin/activate"
fi
exec python3 verification/oracle/scripts/run_ras_reference.py \
  --scenario verification/oracle/scenarios/beaver_unsteady_linked.json \
  --skip-ras-run \
  --verify \
  "$@"
