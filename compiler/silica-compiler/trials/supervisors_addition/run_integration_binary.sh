#!/bin/bash
# Integrate harness: write <base>.sout = merged stdout + stderr from the trial + exit status on the last line.
# Same shape as other trials' Makefiles: "{ ./base …; echo $?; } >base.sout 2>&1".
#
# Important: do NOT use " ./exe >out 2>&1; echo $? >>out " — that closes out after the binary and reopens for
# append, which can reorder or lose bytes (e.g. Phase F unwind _write(2,…) vs stdout) under thread teardown.
# One braced group + one redirect keeps a single open file description for the whole run.
#
# wait_for_exit/0 (Phase I): returns when stdin sees a line that is exactly `exit` (CR-trimmed) **or** when the
# root actor registered at first top-level spawn() dies (`alive == 0`). If stdin already holds the line `exit`
# when `wait_for_exit` polls (integrate here-doc), that line can be consumed before async/unsynchronized work
# finishes—empty `.sout`, or SIGBUS under teardown. Trials listed in integrate_exit_after_done_basenames.txt use
# `run_integration_exit_after_marker.py` (PTY + send `exit` only after a full `done` line appears on output).

set -u
base="${1:?usage: run_integration_binary.sh <basename>}"
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here" || exit 1
exe="./${base}"
if test ! -x "$exe"; then
  echo "run_integration_binary.sh: missing or not executable: $exe" >&2
  exit 1
fi

out="${base}.sout"
marker_helper="${here}/run_integration_exit_after_marker.py"
done_list="${here}/integrate_exit_after_done_basenames.txt"

use_done_marker=0
if test -f "$done_list" && command -v python3 >/dev/null 2>&1 && test -f "$marker_helper"; then
  if grep -v '^#' "$done_list" 2>/dev/null | grep -F -x -q -- "$base"; then
    use_done_marker=1
  fi
fi

if [ "$use_done_marker" -eq 1 ]; then
  python3 "$marker_helper" "$here" "$base" "$out" done
else
  set +e
  {
    "$exe" <<'EOF'
exit
EOF
    echo $?
  } >"$out" 2>&1
  set -e
fi
