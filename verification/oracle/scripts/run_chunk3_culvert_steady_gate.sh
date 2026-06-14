#!/usr/bin/env bash
set -eu
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT"
python3 verification/oracle/scripts/run_chunk3_culvert_steady_gate.py
