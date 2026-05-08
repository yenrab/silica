#!/usr/bin/env bash
# Compare integrate capture <base>.sout to golden <base>.scout in order, while normalizing runtime fields
# that are inherently unstable across runs.
#
# Usage: compare_scout_normalized.sh <path/to/.sout> <path/to/.scout>
# Exit 0 if equal after normalization; 1 otherwise.
#
# Normalization:
#   - CR stripped from PTY / CRLF captures before trimming
#   - trailing whitespace stripped per line, and blank lines dropped (roughly aligned with diff -Bw)
#   - actor_id / supervisor_acb pointer hex normalized to 0x0 (ASLR)
#   - agent_type_atom decimal normalized to 0 (intern table index varies by registration order)

set -euo pipefail
sout="${1:?}"
scout="${2:?}"

tmp_a=
tmp_b=
cleanup() {
  rm -f "${tmp_a:-}" "${tmp_b:-}"
}
trap cleanup EXIT

tmp_a="$(mktemp -t silicascout.XXXXXXXX)"
tmp_b="$(mktemp -t silicascout.XXXXXXXX)"

norm_file() {
  local f=$1
  sed $'s/\r$//' "$f" \
    | sed 's/[[:space:]]*$//' \
    | sed '/^$/d' \
    | sed -E 's/^(actor_id: )0x[0-9A-Fa-f]+/\10x0/; s/^(supervisor_acb: )0x[0-9A-Fa-f]+/\10x0/; s/^(agent_type_atom: )[0-9]+$/\10/'
}

norm_file "$sout" >"$tmp_a"
norm_file "$scout" >"$tmp_b"

if cmp -s "$tmp_a" "$tmp_b"; then
  exit 0
fi

diff -u -L "$scout" -L "$sout" "$tmp_b" "$tmp_a" >&2 || true
exit 1
