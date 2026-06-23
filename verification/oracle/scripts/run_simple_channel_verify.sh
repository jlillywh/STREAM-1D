#!/usr/bin/env bash
# Simple trapezoidal channel — automated HEC-RAS headless + STREAM-1D parity.
set -eu
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT"
if [ -f .venv/bin/activate ]; then
  # shellcheck source=/dev/null
  source .venv/bin/activate
fi
python3 verification/oracle/scripts/run_simple_channel_hecras_parity.py "$@"
