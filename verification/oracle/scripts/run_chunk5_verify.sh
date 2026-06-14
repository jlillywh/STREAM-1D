#!/usr/bin/env bash
# Chunk 5 verification runner (LF line endings). Log: /tmp/chunk5_verify.log
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"
LOG=/tmp/chunk5_verify.log
exec > >(tee "$LOG") 2>&1

echo "=== cargo test ==="
for t in \
  conspan_culvert_interval \
  test_unsteady_implicit_culvert \
  test_unsteady_implicit_conspan_mild \
  test_structure_coupling_diagnostics_mode2 \
  culvert_headwater_residual \
  conspan_arch_implicit
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

echo "=== warm start ==="
python3 verification/oracle/scripts/test_conspan_unsteady_warm_start.py

echo "=== diagnose implicit ==="
python3 verification/oracle/scripts/diagnose_conspan_implicit.py

echo "=== oracle mode 0 ==="
bash verification/oracle/run_oracle.sh --scenario verification/oracle/scenarios/conspan_unsteady_mild_linked.json

echo "=== oracle mode 2 ==="
bash verification/oracle/run_oracle.sh --scenario verification/oracle/scenarios/conspan_unsteady_mild_implicit_linked.json

echo "=== DONE ==="
