#!/usr/bin/env bash
# ConSpan Q-ramp — dense WSEL matrix (all cross sections × 4 hr checkpoints).
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENGINE="$(cd "$ROOT/../.." && pwd)"
cd "$ENGINE"

SCENARIO="verification/oracle/scenarios/conspan_unsteady_ramp_matrix.json"
REF="verification/oracle/projects/conspan/reference_wsel_timeseries_ramp_full.json"
HDF="${CONSPAN_HDF:-$ENGINE/verification/oracle/projects/conspan/ConSpan.p08.hdf}"

ALL_RMS="20.535,20.422,20.308,20.251,20.238,20.227,20.208,20.189,20.095,20.0"
TIMES="0,4,8,12,16,20,24,28,32,36,40,44,48"

echo "=== Extract WSEL timeseries from HDF (if available) ==="
if [[ -f "$HDF" ]]; then
  PYTHONPATH=python python3 verification/oracle/scripts/extract_conspan_ramp_reference.py \
    --hdf "$HDF" \
    --scenario "$SCENARIO" \
    --output "$REF" \
    --checkpoints-rm "$ALL_RMS" \
    --time-checkpoints-hr "$TIMES"
else
  echo "SKIP extract: HDF not found at $HDF"
  if [[ ! -f "$REF" ]]; then
    echo "ERROR: no bundled full reference at $REF — set CONSPAN_HDF or extract manually."
    exit 1
  fi
fi

echo "=== Linked verify (matrix) ==="
PYTHONPATH=python python3 verification/oracle/run_linked_verify.py \
  --scenario "$SCENARIO" \
  --format matrix \
  --reference-file "$REF"
