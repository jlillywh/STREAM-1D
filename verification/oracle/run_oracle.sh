#!/usr/bin/env bash
# Linked HEC-RAS oracle — optional; requires Python stream1d extension.
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

echo "== STREAM-1D linked HEC-RAS oracle =="
echo

if [[ -f "$ROOT/.venv/bin/activate" ]]; then
  # shellcheck disable=SC1091
  source "$ROOT/.venv/bin/activate"
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "SKIP: python3 not found"
  exit 0
fi

_run_import_check() {
  python3 -c "import stream1d" 2>/dev/null \
    || PYTHONPATH=python python3 -c "import stream1d" 2>/dev/null
}

if ! _run_import_check; then
  echo "SKIP: stream1d is not available."
  echo
  echo "  From repo root, build once:"
  echo "    python3 -m venv .venv"
  echo "    source .venv/bin/activate"
  echo "    pip install maturin pytest"
  echo "    maturin develop --features python"
  echo
  echo "  Then test:"
  echo "    python3 -c \"import stream1d; print('ok')\""
  echo "    # or before build completes:"
  echo "    PYTHONPATH=python python3 -c \"import stream1d; print('ok')\""
  exit 0
fi

if python3 -c "import stream1d" 2>/dev/null; then
  python3 verification/oracle/run_linked_verify.py "$@"
else
  PYTHONPATH=python python3 verification/oracle/run_linked_verify.py "$@"
fi
