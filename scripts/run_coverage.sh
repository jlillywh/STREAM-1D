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
  wsl_linux_root() {
    local p="$1"
    p="${p//\\//}"
    if [[ "$p" == //wsl.localhost/* ]]; then
      printf '/%s\n' "$(echo "${p#//wsl.localhost/}" | cut -d/ -f2-)"
    elif [[ "$p" == /home/* ]]; then
      printf '%s\n' "$p"
    elif command -v wslpath >/dev/null 2>&1; then
      wslpath -u "$p"
    else
      printf '%s\n' "$p"
    fi
  }
  WSL_BIN=""
  if command -v wsl.exe >/dev/null 2>&1; then
    WSL_BIN=wsl.exe
  elif command -v wsl >/dev/null 2>&1; then
    WSL_BIN=wsl
  fi
  if [[ -n "$WSL_BIN" ]]; then
    LINUX_ROOT="$(wsl_linux_root "$ROOT")"
    WSL_ARGS=()
    if [[ "$PRE_COMMIT" == true ]]; then
      WSL_ARGS+=(--pre-commit)
    fi
    echo "coverage: delegating to WSL ($LINUX_ROOT)"
    exec "$WSL_BIN" -d Ubuntu bash -lc "cd $(printf '%q' "$LINUX_ROOT") && export PATH=\"\${HOME}/.cargo/bin:\${PATH}\" && bash scripts/run_coverage.sh ${WSL_ARGS[*]}"
  fi
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
