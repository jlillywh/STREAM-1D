#!/usr/bin/env bash
# Chunk 1 ramp transient — friction-slope DS (Plan 04, WSL/Linux).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT"
export PYTHONPATH="${ROOT}/python${PYTHONPATH:+:${PYTHONPATH}}"
SCENARIO="${ROOT}/verification/oracle/scenarios/simple_channel_ramp_unsteady_linked.json"
echo "=== simple_channel ramp verify (friction-slope DS) ==="
python3 verification/oracle/scripts/smoke_simple_channel_ramp_parse.py
python3 verification/oracle/run_linked_verify.py --scenario "$SCENARIO"
