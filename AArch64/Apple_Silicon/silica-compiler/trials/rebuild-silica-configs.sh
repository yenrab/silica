#!/usr/bin/env bash
# Regenerate silica.config in each trial subdirectory from local *.silica files.
# Same content that Make integrate uses: one filename per line.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
for d in "$ROOT"/*/; do
  [[ -d "$d" ]] || continue
  base="$(basename "$d")"
  # Skip dirs that do not use a shared file list this way
  if [[ "$base" == "error_enforcement_addition" ]]; then
    continue
  fi
  (cd "$d" && ls -1 *.silica 2>/dev/null | grep '\.silica$' > silica.config || true)
  n="$(wc -l < "$d/silica.config" 2>/dev/null | tr -d ' ')"
  echo "$base: silica.config ($n sources)"
done
