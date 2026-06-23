#!/usr/bin/env bash
# Chunk 6 verification runner (LF line endings). Log: /tmp/chunk6_verify.log
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"
LOG=/tmp/chunk6_verify.log
exec > >(tee "$LOG") 2>&1

echo "=== cargo test (bridge implicit) ==="
for t in \
  bridge_headwater_implicit \
  test_unsteady_implicit_bridge \
  test_structure_coupling_diagnostics_mode2_bridge \
  test_unsteady_implicit_bridge_tw_ramp
do
  cargo test --lib "$t"
done

echo "=== maturin develop (required after Rust changes) ==="
if [ -f .venv/bin/activate ]; then
  # shellcheck source=/dev/null
  source .venv/bin/activate
else
  echo "ERROR: create venv first: python3 -m venv .venv && source .venv/bin/activate" >&2
  exit 1
fi
maturin develop --features python

echo "=== capture reference (mode 0 baseline) ==="
python3 verification/oracle/scripts/capture_bridge_mild_reference.py

echo "=== smoke ==="
python3 verification/oracle/scripts/smoke_bridge_mild_parse.py

echo "=== warm start (WSPRO + Yarnell) ==="
python3 verification/oracle/scripts/test_bridge_unsteady_warm_start.py

echo "=== diagnose implicit ==="
python3 verification/oracle/scripts/diagnose_bridge_implicit.py

echo "=== oracle mode 0 ==="
bash verification/oracle/run_oracle.sh --scenario verification/oracle/scenarios/bridge_mild_unsteady_linked.json

echo "=== oracle mode 2 ==="
bash verification/oracle/run_oracle.sh --scenario verification/oracle/scenarios/bridge_mild_unsteady_implicit_linked.json

echo "=== oracle mode 2 WSPRO ==="
bash verification/oracle/run_oracle.sh --scenario verification/oracle/scenarios/bridge_mild_wspro_unsteady_implicit_linked.json

echo "=== DONE ==="
