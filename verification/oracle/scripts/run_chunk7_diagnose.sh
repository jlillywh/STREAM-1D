#!/usr/bin/env bash
# Chunk 7 optional diagnostics (LF). Does not implement mode 1.
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

if [ -f .venv/bin/activate ]; then
  # shellcheck source=/dev/null
  source .venv/bin/activate
fi

echo "=== face lag (stiff pulse, mode 0 vs 2) ==="
python3 verification/oracle/scripts/diagnose_face_lag.py

echo "=== mass budget (Rust) ==="
cargo test --lib test_unsteady_structure_interval_mass_budget -- --nocapture

echo "=== multi-structure order (Rust) ==="
cargo test --lib test_build_coupled_structure_order_modes

echo "=== DONE (Chunk 7 diagnostic — mode 1 not shipped) ==="
