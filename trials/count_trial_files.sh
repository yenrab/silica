#!/usr/bin/env bash
# Count all .silica files under this directory (trials) and its subdirectories.
#
# Trial sources may be symlinks (ordered_data_structures/*/lib/*.silica point at
# stdlib sources), so symlinks are counted alongside regular files. Links are not
# followed while walking (ffi_addition has self-referential directory links), and
# per-trial cache directories (.stdlib_cache, .trial_cache) are skipped so cached
# copies are not counted twice.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
count="$(find "$SCRIPT_DIR" \( -name '.stdlib_cache' -o -name '.trial_cache' \) -prune -o \( -type f -o -type l \) -name '*.silica' -print | wc -l | tr -d ' ')"

echo "$count"
