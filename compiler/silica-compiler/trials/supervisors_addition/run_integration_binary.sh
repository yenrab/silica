#!/bin/bash
# Integrate harness helper: run <base>, capture stdout+stderr into <base>.sout, append exit status (same layout as
# "{ ./base 2>&1; echo $?; }"). For binaries that call wait_for_exit/0, wait briefly then send the literal line
# "exit" on stdin so the runtime read loop returns (no SIGTERM).

set -u
base="${1:?usage: run_integration_binary.sh <basename>}"
here="$(dirname "$0")"
cd "$here" || exit 1
exe="./${base}"
if test ! -x "$exe"; then
  echo "run_integration_binary.sh: missing or not executable: $exe" >&2
  exit 1
fi

out="${base}.sout"

set +e
{ sleep 3; printf '%s\n' exit; } | "$exe" >"$out" 2>&1
ec=${PIPESTATUS[1]}
set -e
echo "$ec" >>"$out"
