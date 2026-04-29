#!/usr/bin/env bash
# Run compare_scout_multiset.sh on <.sout> against the first matching <.scout> in the argument list.
# Usage: compare_scout_multiset_oneof.sh <path/to/.sout> <path/to/.scout> [.scout.alt ...]
# If none match, prints unified diffs (diff -u style) against each golden on stderr.
set -euo pipefail
sout="${1:?}"
shift
[ "$#" -ge 1 ] || { echo "compare_scout_multiset_oneof.sh: need at least one .scout" >&2; exit 2; }
here="$(cd "$(dirname "$0")" && pwd)"
for scout in "$@"; do
  if "$here/compare_scout_multiset.sh" "$sout" "$scout" 2>/dev/null; then
    exit 0
  fi
done
echo "compare_scout_multiset_oneof.sh: .sout did not match any golden ($*)" >&2
for scout in "$@"; do
  "$here/compare_scout_multiset.sh" "$sout" "$scout" || true
done
exit 1
