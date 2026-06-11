#!/usr/bin/env bash
# Run all external-source verification suites (from repo root).
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== STREAM-1D verification (external golden / HEC-RAS) =="
echo

cargo test --test culvert_hecras_verification
cargo test --test bridge_abutment_hecras_verification
cargo test --test bridge_bu_bd_hecras_verification
cargo test --test bridge_high_flow_hecras_verification
cargo test --test bridge_roadway_embankment_verification
cargo test --test bridge_guide_bank_contraction_verification
cargo test --test bridge_friction_weighting_hecras_verification
cargo test --test bridge_opening_alignment_verification

if command -v python3 >/dev/null 2>&1; then
  echo
  echo "== Python ConSpan culvert profiles (optional; requires maturin develop) =="
  if PYTHONPATH=python python3 -c "import stream1d" 2>/dev/null; then
    PYTHONPATH=python python3 python/test_hecras_culvert_verification.py
  else
    echo "SKIP: stream1d Python extension not built — run: maturin develop --features python"
  fi
fi

echo
echo "All verification suites passed."
