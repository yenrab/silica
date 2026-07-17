#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: compare_scout_normalized.sh <actual.sout> <golden.scout>" >&2
  exit 2
fi

actual="$1"
golden="$2"

normalize_pointer_lines() {
  awk '
    {
      gsub(/\r/, "", $0)
    }
    /^[[:space:]]*actor_id:[[:space:]]*0x[0-9a-fA-F]+$/ {
      print "actor_id:        <PTR>"
      next
    }
    /^[[:space:]]*supervisor_acb:[[:space:]]*0x[0-9a-fA-F]+$/ {
      print "supervisor_acb:  <PTR>"
      next
    }
    { print }
  ' "$1"
}

tmp_actual="$(mktemp)"
tmp_golden="$(mktemp)"
trap 'rm -f "$tmp_actual" "$tmp_golden"' EXIT

normalize_pointer_lines "$actual" > "$tmp_actual"
normalize_pointer_lines "$golden" > "$tmp_golden"

diff -Bw "$tmp_actual" "$tmp_golden"
