#!/usr/bin/env bash
# Build stream1d and run python/stream1d_verification.ipynb (same steps as CI).
# Usage:
#   bash scripts/run_verification_notebook.sh          # headless execute
#   bash scripts/run_verification_notebook.sh --serve  # jupyter notebook UI
# On WSL/Windows, prefer: python scripts/run_verification_notebook.py
set -eu
exec python3 "$(dirname "$0")/run_verification_notebook.py" "$@"
