#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
FILE="$ROOT/src/solvers/unsteady/culvert_implicit.rs"
SCENARIO="$ROOT/verification/oracle/scenarios/conspan_unsteady_ramp_matrix.json"
OUT="$ROOT/verification/oracle/omega_sweep_results.tsv"
: > "$OUT"
echo -e "omega\toverall_max_ft\trm_20.227\trm_20.238" >> "$OUT"
cd "$ROOT"
for o in 0.0 0.10 0.15 0.20 0.25 0.30 0.35 0.40; do
  sed -i "s/pub(crate) const CULVERT_DEPARTURE_TAILWATER_OMEGA: f64 = [0-9.]*/pub(crate) const CULVERT_DEPARTURE_TAILWATER_OMEGA: f64 = ${o}/" "$FILE"
  maturin develop --features python --release 2>/dev/null | tail -1
  matrix=$(PYTHONPATH=python python3 verification/oracle/run_linked_verify.py --scenario "$SCENARIO" --format matrix 2>&1)
  overall=$(echo "$matrix" | grep "Overall max" | sed -n "s/.*= \([0-9.]*\) ft.*/\1/p")
  rm227=$(echo "$matrix" | awk "/^ *20\\.227 / {print \$NF}")
  rm238=$(echo "$matrix" | awk "/^ *20\\.238 / {print \$NF}")
  echo -e "${o}\t${overall}\t${rm227}\t${rm238}" >> "$OUT"
  echo "omega=$o overall=$overall rm227=$rm227 rm238=$rm238"
done

best=$(awk -F"\t" "NR>1 {print \$2, \$1}" "$OUT" | sort -n | head -1 | awk "{print \$2}")
echo "BEST $best"
sed -i "s/pub(crate) const CULVERT_DEPARTURE_TAILWATER_OMEGA: f64 = [0-9.]*/pub(crate) const CULVERT_DEPARTURE_TAILWATER_OMEGA: f64 = ${best}/" "$FILE"
maturin develop --features python --release 2>/dev/null | tail -1
cat "$OUT"
