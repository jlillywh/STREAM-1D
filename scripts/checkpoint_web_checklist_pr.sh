#!/usr/bin/env bash
# Commit implicit coupling checklist in lillywhite_web (separate repo).
set -euo pipefail

WEB_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../lillywhite_web" && pwd)"
FILE="$WEB_ROOT/streams1d/docs/UNSTEADY_IMPLICIT_COUPLING_CHECKLIST.md"

if [[ ! -f "$FILE" ]]; then
  echo "Checklist not found: $FILE" >&2
  exit 1
fi

cd "$WEB_ROOT"
echo "=== git status (lillywhite_web) ==="
git status -sb

git add "$FILE"
if git diff --cached --quiet; then
  echo "Nothing to commit for checklist."
else
  git commit -m "$(cat <<'EOF'
Add unsteady implicit structure coupling checklist.

Working doc for Phase 5 engine work (Preissmann hook, culvert/bridge
implicit coupling) and streams1d WASM integration follow-up.
EOF
)"
fi

BRANCH="$(git branch --show-current)"
if git rev-parse --verify "@{u}" >/dev/null 2>&1; then
  git push
else
  git push -u origin "$BRANCH"
fi

if ! gh pr view --json url -q .url 2>/dev/null; then
  gh pr create --title "Add unsteady implicit coupling checklist" --body "$(cat <<'EOF'
## Summary

- Adds `streams1d/docs/UNSTEADY_IMPLICIT_COUPLING_CHECKLIST.md` — phased checklist for engine Phase 5 implicit culvert/bridge coupling and web integration.

## Test plan

- [ ] Doc review only (no code changes)
EOF
)"
fi

gh pr view --json url -q .url
