#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: compare_scout_sorted.sh <actual.sout> <golden.scout>" >&2
  exit 2
fi

actual="$1"
golden="$2"

sort_normalized_lines() {
  awk '
    {
      gsub(/\r/, "", $0)
      line = $0
      sub(/[ \t]+$/, "", line)
      if (line == "") next
      if (line ~ /^[[:space:]]*actor_id:[[:space:]]*0x[0-9a-fA-F]+$/) {
        print "actor_id:        <PTR>"
        next
      }
      if (line ~ /^[[:space:]]*supervisor_acb:[[:space:]]*0x[0-9a-fA-F]+$/) {
        print "supervisor_acb:  <PTR>"
        next
      }
      # A call stack frame names a linker symbol, and Mach-O decorates those with a leading
      # underscore while ELF does not. The same program therefore prints _actor_thread_main on
      # macOS and actor_thread_main on Linux, so the leading underscore is dropped before the
      # comparison and one golden serves both targets.
      if (line ~ /^[[:space:]]*#[0-9]+[[:space:]]+_[A-Za-z_]/) {
        sub(/_/, "", line)
        print line
        next
      }
      print line
    }
  ' "$1" | LC_ALL=C sort
}

tmp_actual="$(mktemp)"
tmp_golden="$(mktemp)"
trap 'rm -f "$tmp_actual" "$tmp_golden"' EXIT

sort_normalized_lines "$actual" > "$tmp_actual"
sort_normalized_lines "$golden" > "$tmp_golden"

diff -u "$tmp_golden" "$tmp_actual"
