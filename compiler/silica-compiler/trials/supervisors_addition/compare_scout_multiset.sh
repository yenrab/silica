#!/usr/bin/env bash
# Compare integrate capture <base>.sout to golden <base>.scout when line *order* (and glued vs split
# Phase F banner lines) may differ but the multiset of logical lines should match.
#
# Usage: compare_scout_multiset.sh <path/to/.sout> <path/to/.scout>
# Exit 0 if equal after normalization; 1 otherwise (prints sorted diff on stderr).
#
# Normalization:
#   - CR stripped from PTY / CRLF captures before trimming
#   - trailing whitespace stripped per line (roughly aligned with diff -Bw)
#   - lines containing "...=== Silica Actor Failure ===..." with a non-empty prefix split into prefix line + banner line
#   - blank lines dropped (integrate output often differs only by extra blanks)
#   - actor_id / supervisor_acb pointer hex normalized to 0x0 (ASLR) so .scout can use stable placeholders
#   - agent_type_atom decimal normalized to 0 (intern table index varies by registration order)

set -euo pipefail
sout="${1:?}"
scout="${2:?}"

mark='=== Silica Actor Failure ==='

norm_file() {
  local f=$1
  sed $'s/\r$//' "$f" | sed 's/[[:space:]]*$//' | while IFS= read -r line || [ -n "${line-}" ]; do
    case "$line" in
      *"$mark"*)
        before="${line%%"$mark"*}"
        after="${line#*"$mark"}"
        if [ -n "$before" ]; then
          printf '%s\n' "$before"
        fi
        printf '%s%s\n' "$mark" "$after"
        ;;
      *)
        printf '%s\n' "$line"
        ;;
    esac
  done | sed '/^$/d'
}

tmp_a=
tmp_b=
cleanup() {
  rm -f "${tmp_a:-}" "${tmp_b:-}"
}
trap cleanup EXIT

tmp_a="$(mktemp -t silicascout.XXXXXXXX)"
tmp_b="$(mktemp -t silicascout.XXXXXXXX)"

norm_file "$sout" \
  | sed -E 's/^(actor_id: )0x[0-9A-Fa-f]+/\10x0/; s/^(supervisor_acb: )0x[0-9A-Fa-f]+/\10x0/; s/^(agent_type_atom: )[0-9]+$/\10/' \
  | LC_ALL=C sort >"$tmp_a"
norm_file "$scout" \
  | sed -E 's/^(actor_id: )0x[0-9A-Fa-f]+/\10x0/; s/^(supervisor_acb: )0x[0-9A-Fa-f]+/\10x0/; s/^(agent_type_atom: )[0-9]+$/\10/' \
  | LC_ALL=C sort >"$tmp_b"

if cmp -s "$tmp_a" "$tmp_b"; then
  exit 0
fi

echo "compare_scout_multiset.sh: multiset or content mismatch (sorted view follows)" >&2
diff -u "$tmp_b" "$tmp_a" >&2 || true
exit 1
