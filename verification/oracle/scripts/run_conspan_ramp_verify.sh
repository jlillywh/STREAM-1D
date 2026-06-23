#!/usr/bin/env bash
# ConSpan Q-ramp unsteady linked verify (p08 HDF reference).
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENGINE="$(cd "$ROOT/../.." && pwd)"
cd "$ENGINE"

HDF="${CONSPAN_HDF:-$ENGINE/verification/oracle/projects/conspan/ConSpan.p08.hdf}"

echo "=== Extract WSEL timeseries from HDF (if available) ==="
if [[ -f "$HDF" ]]; then
  PYTHONPATH=python python3 verification/oracle/scripts/extract_conspan_ramp_reference.py \
    --hdf "$HDF"
else
  echo "SKIP extract: HDF not found at $HDF (using bundled reference JSON)"
fi

echo "=== Linked verify ==="
PYTHONPATH=python python3 verification/oracle/run_linked_verify.py \
  --scenario verification/oracle/scenarios/conspan_unsteady_ramp_linked.json
