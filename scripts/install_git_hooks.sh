#!/usr/bin/env bash
# Point this repo at tracked hooks in .githooks/ (run once per clone).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

chmod +x scripts/run_coverage.sh
chmod +x .githooks/pre-commit

git config core.hooksPath .githooks

echo "Git hooks installed."
echo "  hooks path: .githooks"
echo "  pre-commit: runs scripts/run_coverage.sh before each commit"
echo ""
echo "Run coverage manually anytime:"
echo "  ./scripts/run_coverage.sh"
