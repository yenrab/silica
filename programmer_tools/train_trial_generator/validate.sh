#!/usr/bin/env bash
# Validate generated trial files with the real per-directory Makefiles.
#   validate.sh <out_root> <area> [<area> ...]
# Builds <scratch>/vsilica/{binaries -> real, compiler -> real, trials/<area>, trials/silica_compiler.mk}
# from the real trial dir plus the generated files, then runs `make integrate` there.
set -uo pipefail
OUT="$1"; shift
ROOT="${SILICA_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
V="${SILICA_GEN_WORK:-${TMPDIR:-/tmp}/silica_train_gen}/vsilica"
mkdir -p "$V/trials"
[ -e "$V/binaries" ] || ln -s "$ROOT/binaries" "$V/binaries"
[ -e "$V/compiler" ] || ln -s "$ROOT/compiler" "$V/compiler"
cp "$ROOT/trials/silica_compiler.mk" "$V/trials/"
[ -f "$ROOT/trials/stdlib_prereq.mk" ] && cp "$ROOT/trials/stdlib_prereq.mk" "$V/trials/"
[ -f "$ROOT/trials/link_lib_deps.mk" ] && cp "$ROOT/trials/link_lib_deps.mk" "$V/trials/"
for area in "$@"; do
  dst="$V/trials/$area"
  rm -rf "$dst"
  mkdir -p "$dst"
  if [ "$area" = "error_enforcement_addition" ]; then
    # top-level single-file trials only; SUB_TRIALS are overridden to empty below
    find "$ROOT/trials/$area" -maxdepth 1 -type f \( -name '*.silica' -o -name '*.golden_fail' -o -name '*.no_golden_fail' -o -name 'Makefile' -o -name 'makefile' \) -exec cp {} "$dst/" \;
  else
    (cd "$ROOT/trials/$area" && find . -type f \( -name '*.silica' -o -name '*.scout' -o -name '*.ascomp' -o -name 'Makefile' -o -name 'makefile' -o -name '*.mk' -o -name '*.wait_for_exit' -o -name '*.scout.*' -o -name '*.sh' -o -name '*.py' -o -name '*.no_golden_fail' -o -name '*.meta' -o -name 'INTEGRATE_PENDING' \) -print0 | while IFS= read -r -d '' f; do mkdir -p "$dst/$(dirname "$f")"; cp "$f" "$dst/$f"; done)
  fi
  if [ -d "$OUT/$area" ]; then
    (cd "$OUT/$area" && find . -type f -print0 | while IFS= read -r -d '' f; do mkdir -p "$dst/$(dirname "$f")"; cp "$f" "$dst/$f"; done)
  fi
  log="$V/$area.integrate.log"
  if [ "$area" = "error_enforcement_addition" ]; then
    (cd "$dst" && make SUB_TRIALS= integrate > "$log" 2>&1); st=$?
  else
    (cd "$dst" && make integrate > "$log" 2>&1); st=$?
  fi
  counts="$(cat "$dst/.integrate_counts" 2>/dev/null || echo '? ?')"
  echo "$area status=$st counts(ok fail)=$counts"
  grep -E '❌❌' "$log" | head -5
done
