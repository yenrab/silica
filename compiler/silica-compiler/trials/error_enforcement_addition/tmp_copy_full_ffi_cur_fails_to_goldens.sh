#!/usr/bin/env bash
set -euo pipefail

trial_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
apply=1

case "${1:-}" in
  --apply)
    apply=1
    ;;
  "")
    ;;
  --dry-run)
    apply=0
    ;;
  *)
    echo "usage: $(basename "$0") [--dry-run|--apply]" >&2
    exit 2
    ;;
esac

first_difference_starts_with_ffi() {
  local golden="$1"
  local cur="$2"
  local first_diff

  first_diff="$(
    { diff "$cur" "$golden" || true; } |
      awk '/^[<>][[:space:]]/ { print; exit }'
  )"

  if [ -z "$first_diff" ]; then
    first_diff="$(
      { diff -u "$cur" "$golden" || true; } |
      awk '
        /^--- / { next }
        /^\+\+\+ / { next }
        /^@@ / { next }
        /^[+-]/ { print; exit }
      '
    )"
  fi

  printf '%s\n' "$first_diff" | grep -Eq '^[<>+-][[:space:]]+FFI[[:space:]]'
}

updated=0
skipped=0

for cur in "$trial_dir"/*.cur_fail; do
  [ -e "$cur" ] || continue

  base="${cur%.cur_fail}"
  golden="$base.golden_fail"
  name="$(basename "$base")"

  if [ ! -f "$golden" ]; then
    echo "skip  $name (no .golden_fail)"
    skipped=$((skipped + 1))
    continue
  fi

  if cmp -s "$cur" "$golden"; then
    echo "same  $name"
  elif first_difference_starts_with_ffi "$golden" "$cur"; then
    if [ "$apply" -eq 1 ]; then
      cp "$cur" "$golden"
      echo "copy  $name.cur_fail -> $name.golden_fail"
    else
      echo "would $name.cur_fail -> $name.golden_fail"
    fi
    updated=$((updated + 1))
  else
    echo "skip  $name (first diff is not FFI progress)"
    skipped=$((skipped + 1))
  fi
done

if [ "$apply" -eq 1 ]; then
  echo "updated $updated golden file(s); skipped $skipped"
else
  echo "dry run: would update $updated golden file(s); skipped $skipped"
  echo "rerun with --apply to copy"
fi
