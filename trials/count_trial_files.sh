#!/usr/bin/env bash
# Count the golden files under this directory (trials) and its subdirectories:
# .ascomp (assembly), .scout (runtime output + exit code) and .golden_fail (expected
# compiler diagnostics). Symlinked goldens are counted alongside regular files
# (ordered_data_structures leaves symlink the shared stdlib .ascomp goldens). Links are
# not followed while walking (ffi_addition has self-referential directory links), and
# per-trial cache directories (.stdlib_cache, .trial_cache) are skipped so cached
# copies are not counted twice.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
count="$(find "$SCRIPT_DIR" \( -name '.stdlib_cache' -o -name '.trial_cache' \) -prune -o \( -type f -o -type l \) \( -name '*.ascomp' -o -name '*.scout' -o -name '*.golden_fail' \) -print | wc -l | tr -d ' ')"

echo "$count"
