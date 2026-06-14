#!/usr/bin/env bash
# Copy linked HEC-RAS project bundles into verification/oracle/projects/.
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENGINE_ROOT="$(cd "$ROOT/../.." && pwd)"
WEB="${LILLYWHITE_WEB_ROOT:-$(cd "$ENGINE_ROOT/../.." && pwd)/lillywhite_web}"

copy_beaver() {
  local src="$WEB/streams1d/hecras_outputs/beaver"
  local dst="$ROOT/projects/beaver"
  if [[ ! -f "$src/beaver.g01" ]]; then
    echo "SKIP beaver: not found at $src (set LILLYWHITE_WEB_ROOT if needed)"
    return 0
  fi
  mkdir -p "$dst"
  cp "$src/beaver.g01" "$src/beaver.u02" "$dst/"
  if [[ -f "$src/beaver.p03" ]]; then
    cp "$src/beaver.p03" "$dst/"
  fi
  # Copy any other plan files present
  for plan in "$src"/beaver.p*; do
    [[ -f "$plan" ]] || continue
    cp "$plan" "$dst/"
  done
  echo "OK beaver -> $dst"
}

copy_beaver
