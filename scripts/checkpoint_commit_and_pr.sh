#!/usr/bin/env bash
# Checkpoint: commit Phase 5.0 refactor + 5.1 design on feat/bridge-ice-debris-v32, push, open PR.
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
else
  git commit -m "$(cat <<'EOF'
Refactor bridge module for Phase 5 implicit coupling prep (5.0 R1–R5).

Split monolithic bridge solver and tests into focused submodules, extract
unsteady structure coupling, and DRY steady/unsteady reach glue. Add Phase
5.1 implicit coupling design doc. Behavior-neutral; full test suite passes.
EOF
)"
  echo "commit created"
fi

echo ""
echo "=== push ==="
git push -u origin HEAD

echo ""
echo "=== create PR (if none exists) ==="
if gh pr view --json url -q .url 2>/dev/null; then
  gh pr view --json url,title -q '"PR already open: " + .url + " — " + .title'
else
  gh pr create --base main --title "Bridge ice/debris (v32) and Phase 5.0 refactor checkpoint" --body "$(cat <<'EOF'
## Summary

- **API v32** — Optional bridge ice/debris modifiers (opening blockage, pier debris, ice thickness, deck ice) with WASM metadata and unit tests.
- **Phase 5.0 (R1–R5)** — Behavior-neutral bridge module refactor: split `bridge.rs` / `bridge_tests.rs` into `src/solvers/bridge/`, extract `reach_coupling`, `unsteady/structure_coupling`, and `bridge/unsteady_coupling` for clearer boundaries before implicit unsteady coupling.
- **Phase 5.1 design** — `docs/development/unsteady_implicit_bridge_coupling.md` (Jacobian vs explicit iteration, staging through 5.2–5.5).

No physics change in the refactor commits; existing bridge/culvert/unsteady behavior preserved.

## Test plan

- [x] `cargo test` — full suite (410 tests)
- [ ] CI / Codecov green on PR
- [ ] WASM contract tests (`cargo test --test wasm_json_contract`)
- [ ] Spot-check: bridge ice/debris unit tests, unsteady inline bridge tests

## Follow-up (not in this PR)

- Phase 5.2 — Preissmann structure-interval hook + culvert implicit
- Web checklist: `lillywhite_web/streams1d/docs/UNSTEADY_IMPLICIT_COUPLING_CHECKLIST.md` (separate repo PR)
EOF
)"
fi

echo ""
echo "=== done ==="
gh pr view --web 2>/dev/null || gh pr view --json url -q .url
