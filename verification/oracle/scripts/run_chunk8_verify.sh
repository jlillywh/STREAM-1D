#!/usr/bin/env bash
# Chunk 8 certification runner (LF line endings). Log: /tmp/chunk8_verify.log
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"
LOG=/tmp/chunk8_verify.log
exec > >(tee "$LOG") 2>&1

echo "=== maturin develop (required for oracle) ==="
if [ -f .venv/bin/activate ]; then
  # shellcheck source=/dev/null
  source .venv/bin/activate
else
  echo "ERROR: create venv first: python3 -m venv .venv && source .venv/bin/activate" >&2
  exit 1
fi
maturin develop --features python

echo "=== smoke parse (g01 + u02 + p03 + mapper) ==="
python3 verification/oracle/scripts/smoke_beaver_parse.py

echo "=== warm start (initial WSEL vs steady @ initial Q) ==="
python3 verification/oracle/scripts/test_beaver_unsteady_warm_start.py

echo "=== Beaver gap table (mode 0 vs 2 vs Observed HWM) ==="
python3 verification/oracle/scripts/diagnose_beaver_unsteady.py

echo "=== linked oracle (mode 2, scenario default) ==="
bash verification/oracle/run_oracle.sh \
  --scenario verification/oracle/scenarios/beaver_unsteady_linked.json

echo "=== DONE (Chunk 8 — certification development) ==="
