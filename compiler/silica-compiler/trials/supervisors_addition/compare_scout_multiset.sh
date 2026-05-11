#!/usr/bin/env bash
# Compare integrate capture <base>.sout to golden <base>.scout when line *order* (and known glued vs
# split concurrent output fragments) may differ but the multiset of logical lines should match.
#
# Usage: compare_scout_multiset.sh <path/to/.sout> <path/to/.scout>
# Exit 0 if equal after normalization; 1 otherwise (prints unified diff on stderr, same style as diff -u).
#
# Normalization:
#   - CR stripped from PTY / CRLF captures before trimming
#   - trailing whitespace stripped per line (roughly aligned with diff -Bw)
#   - lines containing "...=== Silica Actor Failure ===..." with a non-empty prefix split into prefix line + banner line
#   - lines containing known async handle_report markers split into prefix line + marker line + suffix line
#   - blank lines dropped (integrate output often differs only by extra blanks)
#   - actor_id / supervisor_acb pointer hex normalized to 0x0 (ASLR) so .scout can use stable placeholders
#   - agent_type_atom decimal normalized to 0 (intern table index varies by registration order)

set -euo pipefail
sout="${1:?}"
scout="${2:?}"

mark='=== Silica Actor Failure ==='
f6_mark='F6_handle_report'
f8_mark='F8_handle_report_banner_ok'
f9_mark='F9_handle_report'

norm_file() {
  local f=$1
  sed $'s/\r$//' "$f" | sed 's/[[:space:]]*$//' | while IFS= read -r line || [ -n "${line-}" ]; do
    case "$line" in
      *"$f6_mark"*)
        before="${line%%"$f6_mark"*}"
        after="${line#*"$f6_mark"}"
        if [ -n "$before" ]; then
          printf '%s\n' "$before"
        fi
        printf '%s\n' "$f6_mark"
        if [ -n "$after" ]; then
          printf '%s\n' "$after"
        fi
        ;;
      *"$f8_mark"*)
        before="${line%%"$f8_mark"*}"
        after="${line#*"$f8_mark"}"
        if [ -n "$before" ]; then
          printf '%s\n' "$before"
        fi
        printf '%s\n' "$f8_mark"
        if [ -n "$after" ]; then
          printf '%s\n' "$after"
        fi
        ;;
      *"$f9_mark"*)
        before="${line%%"$f9_mark"*}"
        after="${line#*"$f9_mark"}"
        if [ -n "$before" ]; then
          printf '%s\n' "$before"
        fi
        printf '%s\n' "$f9_mark"
        if [ -n "$after" ]; then
          printf '%s\n' "$after"
        fi
        ;;
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
  | sed -E \
      -e 's/^actor_id:[[:space:]]+0x[0-9A-Fa-f]+/actor_id: 0x0/' \
      -e 's/^supervisor_acb:[[:space:]]+0x[0-9A-Fa-f]+/supervisor_acb: 0x0/' \
      -e 's/^agent_type_atom:[[:space:]]+[0-9]+$/agent_type_atom: 0/' \
      -e 's/^reason_tag:[[:space:]]+/reason_tag: /' \
      -e 's/^  #0  .*/  #0  <behavior>/' \
  | LC_ALL=C sort >"$tmp_a"
norm_file "$scout" \
  | sed -E \
      -e 's/^actor_id:[[:space:]]+0x[0-9A-Fa-f]+/actor_id: 0x0/' \
      -e 's/^supervisor_acb:[[:space:]]+0x[0-9A-Fa-f]+/supervisor_acb: 0x0/' \
      -e 's/^agent_type_atom:[[:space:]]+[0-9]+$/agent_type_atom: 0/' \
      -e 's/^reason_tag:[[:space:]]+/reason_tag: /' \
      -e 's/^  #0  .*/  #0  <behavior>/' \
  | LC_ALL=C sort >"$tmp_b"

if cmp -s "$tmp_a" "$tmp_b"; then
  exit 0
fi

# Report like `diff -u`: golden first (---), actual second (+++); paths are labels only (content is normalized sorted).
diff -u -L "$scout" -L "$sout" "$tmp_b" "$tmp_a" >&2 || true
exit 1
