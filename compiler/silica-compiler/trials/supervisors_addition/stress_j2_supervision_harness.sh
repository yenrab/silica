#!/usr/bin/env bash
# Phase J2 — repeatable supervision stress + integrate gate (see design_documents/supervisors_j2_runtime_hardening.md).
#
# Runs:
#   1. make integrate (× INTEGRATE_ROUNDS) in this directory — full asm diff + binary run + .scout checks
#   2. stress_j1_supervision_batch.sh — loops phase_e4e_one_for_one + phase_e4f_rest_for_one (STRESS_ITERS per pair)
#
# Requires: macOS toolchain, silica-compiler on PATH relative to Makefile (../../src/silica-compiler), built binaries.
#
# Usage:
#   ./stress_j2_supervision_harness.sh
#   INTEGRATE_ROUNDS=3 STRESS_ITERS=50 J2_LOG=/tmp/j2.log ./stress_j2_supervision_harness.sh

set -u
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here" || exit 1

log() { echo "$@"; if test -n "${J2_LOG:-}"; then printf '%s %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$*" >>"$J2_LOG"; fi; }

INTEGRATE_ROUNDS="${INTEGRATE_ROUNDS:-1}"
STRESS_ITERS="${STRESS_ITERS:-25}"

if ! command -v make >/dev/null 2>&1; then
  log "stress_j2_supervision_harness.sh: make not found"
  exit 1
fi

log "=== Phase J2 harness: INTEGRATE_ROUNDS=$INTEGRATE_ROUNDS STRESS_ITERS=$STRESS_ITERS ==="

r=1
while test "$r" -le "$INTEGRATE_ROUNDS"; do
  log "--- make integrate ($r / $INTEGRATE_ROUNDS) ---"
  if ! make integrate; then
    log "FAIL: make integrate (round $r)"
    exit 1
  fi
  if test -f .integrate_counts; then
    log "  .integrate_counts: $(tr '\n' ' ' < .integrate_counts)"
  fi
  r=$((r + 1))
done

sj="${here}/stress_j1_supervision_batch.sh"
if test ! -x "$sj"; then
  chmod +x "$sj" 2>/dev/null || true
fi
if test ! -x "$sj"; then
  log "FAIL: missing or not executable: $sj"
  exit 1
fi

log "--- stress_j1_supervision_batch.sh $STRESS_ITERS ---"
if ! "$sj" "$STRESS_ITERS"; then
  log "FAIL: stress batch"
  exit 1
fi

log "Phase J2 harness: PASS."
exit 0
