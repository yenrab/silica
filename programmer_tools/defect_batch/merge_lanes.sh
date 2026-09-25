#!/bin/bash
# Merge the four fix lanes onto main AFTER the directory move is committed.
# Dry by default: prints what each merge would do. Pass --apply to merge for real.
# Order is least-entangled first, so a conflict lands in the smallest tree possible.
set -uo pipefail
cd /Volumes/2T/silica || exit 1
APPLY=0; [ "${1:-}" = "--apply" ] && APPLY=1

LANES="worktree-agent-a0f4cd3fb29ebb8e2 worktree-agent-a27ee92d458acdfe1 worktree-agent-ae684c103739140b4 worktree-agent-a4e2fd306e14f8748"
#      frontend (5 defects)           diagnostics (5 + parser)        codegen (2 emitters)          integrate (4 parked lanes, largest)

if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
    echo "main has uncommitted tracked changes; commit the directory move first." >&2
    git status --porcelain --untracked-files=no | head -5 >&2
    exit 1
fi

for b in $LANES; do
    git rev-parse --verify "$b" >/dev/null 2>&1 || { echo "missing branch: $b"; continue; }
    echo "=== $b"
    git log --oneline "main..$b" | head -3
    echo "  files: $(git diff --name-only main..."$b" | wc -l | tr -d ' ')"
    if [ "$APPLY" = 1 ]; then
        if git merge --no-ff -m "merge $b: compiler defect fixes" "$b"; then
            echo "  merged"
        else
            echo "  CONFLICTS:"; git diff --name-only --diff-filter=U | sed 's/^/    /'
            echo "  resolve, then: git commit && bash $0 --apply"
            exit 2
        fi
    else
        git merge --no-commit --no-ff "$b" >/dev/null 2>&1
        conf=$(git diff --name-only --diff-filter=U | wc -l | tr -d ' ')
        git merge --abort 2>/dev/null
        echo "  would conflict in $conf file(s)"
    fi
done
echo "done"
