#!/usr/bin/env bash
# Run tests with LLVM coverage (same as CI). Exits non-zero if tests fail.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export PATH="${HOME}/.cargo/bin:${PATH}"

PRE_COMMIT=false
if [[ "${1:-}" == "--pre-commit" ]]; then
  PRE_COMMIT=true
fi

if $PRE_COMMIT; then
  if ! git diff --cached --name-only | grep -qE '^(src/|tests/|Cargo\.(toml|lock)|\.github/workflows/)'; then
    echo "coverage: skipping (no staged Rust/CI changes)"
    exit 0
  fi
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "coverage: error — cargo not found in PATH" >&2
  exit 1
fi

if ! cargo llvm-cov --version >/dev/null 2>&1; then
  echo "coverage: installing cargo-llvm-cov (one-time)..."
  cargo install cargo-llvm-cov
fi

if ! rustup component list --installed 2>/dev/null | grep -q 'llvm-tools-preview'; then
  echo "coverage: installing llvm-tools-preview (one-time)..."
  rustup component add llvm-tools-preview
fi

echo "=== STREAM-1D coverage + tests ==="
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
echo "=== lcov.info written (gitignored) ==="
