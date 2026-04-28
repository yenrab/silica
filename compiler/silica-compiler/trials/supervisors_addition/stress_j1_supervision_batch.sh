#!/usr/bin/env bash
# Phase J1 optional stress: repeat heavier supervision trials (same runtime path as flake repros — see
# supervisors_j1_invariant_audit.md §1 J1.7). Build trial binaries before running:
#   (from repo / integrate) produce phase_e4e_one_for_all and phase_e4f_rest_for_one in this directory.
#
# Usage: ./stress_j1_supervision_batch.sh [iterations]
# Default iterations: 25

set -u
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here" || exit 1

n="${1:-25}"
e4e="./phase_e4e_one_for_all"
e4f="./phase_e4f_rest_for_one"

for exe in "$e4e" "$e4f"; do
  if test ! -x "$exe"; then
    echo "stress_j1_supervision_batch.sh: missing executable: $exe" >&2
    echo "Build the supervisors_addition trials first." >&2
    exit 1
  fi
done

echo "Phase J1 batch: $n iterations each of phase_e4e_one_for_all and phase_e4f_rest_for_one"
fail=0
for i in $(seq 1 "$n"); do
  if ! "$e4e" >/dev/null 2>&1; then
    echo "FAIL iteration $i: $e4e" >&2
    fail=1
    break
  fi
  if ! "$e4f" >/dev/null 2>&1; then
    echo "FAIL iteration $i: $e4f" >&2
    fail=1
    break
  fi
  if (( i % 5 == 0 )); then
    echo "  ... $i / $n ok"
  fi
done

if (( fail != 0 )); then
  exit 1
fi
echo "stress_j1_supervision_batch.sh: all ${n}×2 runs exited zero."
exit 0
