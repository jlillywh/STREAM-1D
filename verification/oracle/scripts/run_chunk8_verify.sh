#!/usr/bin/env bash
# Beaver bridge restart runner — layered diagnostics (not a certification gate).
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

echo "=== g01 parser regression (BR Coef + deck chords) ==="
python3 verification/oracle/scripts/test_beaver_g01_parser.py

echo "=== Layer 1 — mapping smoke (g01 + u02 + p03 + mapper) ==="
python3 verification/oracle/scripts/smoke_beaver_parse.py

echo "=== internal — warm start (initial WSEL vs steady @ initial Q) ==="
python3 verification/oracle/scripts/test_beaver_unsteady_warm_start.py

echo "=== layered restart diagnostic (mapping + steady + unsteady) ==="
python3 verification/oracle/scripts/diagnose_beaver_restart.py

echo "=== DONE (Beaver bridge restart — diagnostic only) ==="
