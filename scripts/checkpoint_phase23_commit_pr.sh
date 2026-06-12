#!/usr/bin/env bash
# Phase 2-3: Preissmann extract + implicit culvert coupling (API v33)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "=== git status ==="
git status -sb

echo ""
echo "=== running tests ==="
cargo test --quiet
echo "tests passed"

echo ""
echo "=== staging ==="
git add -A
git status -sb

if git diff --cached --quiet; then
  echo "Nothing staged to commit (working tree clean?)."
  exit 0
fi

git commit -m "$(cat <<'EOF'
Add Preissmann implicit culvert coupling (API v33, Phase 2-3).

Extract Preissmann step with structure-interval hooks, wire
unsteady_structure_coupling_mode, and implement inlet-control culvert
residual injection in mode 2 with explicit post-step fallback when the
face residual is not yet satisfied.
EOF
)"

echo ""
echo "=== push ==="
git push -u origin HEAD

echo ""
echo "=== create or show PR ==="
if gh pr view --json url -q .url 2>/dev/null; then
  echo "Existing PR:"
  gh pr view --json url,title -q '"\(.url) — \(.title)"'
else
  gh pr create --base main --title "API v33: Preissmann implicit culvert coupling (Phase 2-3)" --body "$(cat <<'EOF'
## Summary

- **Phase 2** — Extract `solve_preissmann_step` to `unsteady/preissmann.rs`; tag culvert/bridge intervals; structure momentum hook; API v33 `unsteady_structure_coupling_mode` (0 = post-step, 2 = implicit).
- **Phase 3** — Inlet-controlled circular/box culvert FHWA headwater residual in mode `2`; explicit post-step fallback when `|R|` is not satisfied at the face.
- **Docs/types** — WASM metadata, TypeScript types, Python binding, `equations.md` update.

## Test plan

- [x] `cargo test` — full lib suite (368+ tests)
- [ ] `cargo test --test wasm_json_contract`
- [ ] CI green on PR
- [ ] Spot-check: `test_unsteady_implicit_culvert_constant_q_matches_steady_hw`, `culvert_headwater_residual_matches_solve_culvert`

## Follow-up

- Phase 4 — bridge implicit residual on same Preissmann hook
- Outlet-control culvert implicit (or keep explicit fallback)
EOF
)"
fi

echo ""
gh pr view --json url -q .url
