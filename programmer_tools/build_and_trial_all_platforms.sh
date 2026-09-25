#!/usr/bin/env bash
# Build Silica for every indicated platform, then run the full trial tree on each of them.
#
#   bash programmer_tools/build_and_trial_all_platforms.sh                 # every platform it knows
#   bash programmer_tools/build_and_trial_all_platforms.sh --list          # the whole plan, then stop
#   bash programmer_tools/build_and_trial_all_platforms.sh --targets "apple_silicon_mac linux_x86_64"
#   bash programmer_tools/build_and_trial_all_platforms.sh --set_targets "apple_silicon_mac linux_aarch64"
#   bash programmer_tools/build_and_trial_all_platforms.sh --local-only --board
#
# This is the two-step run in one command:
#   1. programmer_tools/build_all_platforms.sh   builds each OS-hosted platform natively on its own
#      machine and takes it to its fixed point, and cross-builds each raw target here.
#   2. programmer_tools/run_trials_all_platforms.sh   runs the full trial tree on each of those
#      platforms with the compiler that was just built there.
# Both steps read the same machine list, so the platforms you name or record once are used twice.
#
# Which platforms, and where their machines are:
#   .silica_build_hosts in the repository root remembers the platforms and their machines. It is
#   written the first time you are asked, used silently after that, gitignored, and must never be
#   committed. Remote machines are reached with public/private key ssh in batch mode, so set the key
#   up first (ssh-keygen, then ssh-copy-id user@host).
#   no flags            the file, if it exists; otherwise every supported platform, asking for each
#   --targets "<list>"  only those platforms, this run only; the file is read but not rewritten
#   --set_targets "<l>" ask for that list and write (or replace) the file
#   --refresh_targets   ask again for the platforms already in the file and rewrite it
#   --remote <t>=<user@host>[:<path>]   one machine on the command line
#   --local-only        this machine and the raw targets, no remote machines
#
# Other flags:
#   --board             also run a raw target's trials on an attached board (off by default)
#   --sync              copy trials, stdlib and project makefiles to each remote machine first
#   --jobs N            parallel jobs for the builds
#   --no-fixpoint       build only; skip gen1, gen2 and the fixed-point check
#   --skip-build        go straight to the trials
#   --skip-trials       build only, and stop
#   --list              print both plans and stop
#   --log-dir DIR       build logs in DIR/build, trial logs and reports in DIR/trials
#
# The trials do not run if the builds fail: a trial result only means something when it came from a
# compiler that built cleanly. Run the trials script on its own if you want them anyway.
#
# The exit status is 0 only when every build and every trial run passed.

set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="${SILICA_REPO:-$(cd "$HERE/.." && pwd)}"
BUILD_SH="$HERE/build_all_platforms.sh"
TRIALS_SH="$HERE/run_trials_all_platforms.sh"

STAMP="$(date '+%Y%m%d-%H%M%S')"
LOG_DIR="${LOG_DIR:-$REPO/../silica_releases/$STAMP}"
DO_BUILD=1
DO_TRIALS=1
LIST_ONLY=0

declare -a BUILD_ARGS=() TRIAL_ARGS=()

usage() {  # every comment line of the header, however long it grows
    awk 'NR>1 && /^#/ { sub(/^# ?/, ""); print; next } NR>1 { exit }' "$0"
    exit "${1:-0}"
}

for f in "$BUILD_SH" "$TRIALS_SH"; do
    [ -f "$f" ] || { echo "missing: $f" >&2; exit 1; }
done

while [ $# -gt 0 ]; do
    case "$1" in
        --targets)
            BUILD_ARGS+=(--targets "$2"); TRIAL_ARGS+=(--targets "$2"); shift ;;
        --set_targets|--set-targets)
            BUILD_ARGS+=(--set_targets "$2"); TRIAL_ARGS+=(--targets "$2"); shift ;;
        --refresh_targets|--refresh-targets)
            BUILD_ARGS+=(--refresh_targets) ;;
        --remote)
            BUILD_ARGS+=(--remote "$2"); TRIAL_ARGS+=(--remote "$2"); shift ;;
        --local-only)
            BUILD_ARGS+=(--local-only); LOCAL_ONLY=1 ;;
        --jobs)        BUILD_ARGS+=(--jobs "$2"); shift ;;
        --no-fixpoint) BUILD_ARGS+=(--no-fixpoint) ;;
        --board)       TRIAL_ARGS+=(--board) ;;
        --sync)        TRIAL_ARGS+=(--sync) ;;
        --skip-build)  DO_BUILD=0 ;;
        --skip-trials) DO_TRIALS=0 ;;
        --list)        LIST_ONLY=1 ;;
        --log-dir)     LOG_DIR="$2"; shift ;;
        -h|--help)     usage 0 ;;
        *)             echo "unknown argument: $1" >&2; usage 1 ;;
    esac
    shift
done

# --local-only means the same thing to the trials: this machine and the board, no ssh.
if [ "${LOCAL_ONLY:-0}" = 1 ]; then
    HOST_TARGET="$(MAKEFLAGS= MAKELEVEL= make -s --no-print-directory \
        -f "$REPO/project_makefiles/platform/platforms.mk" host-emit-target 2>/dev/null | head -1)"
    [ -n "$HOST_TARGET" ] && TRIAL_ARGS+=(--targets "$HOST_TARGET")
fi

if [ "$LIST_ONLY" = 1 ]; then
    echo "=== build plan"
    bash "$BUILD_SH" --list ${BUILD_ARGS[@]+"${BUILD_ARGS[@]}"}
    echo
    echo "=== trial plan"
    bash "$TRIALS_SH" --list ${TRIAL_ARGS[@]+"${TRIAL_ARGS[@]}"}
    exit 0
fi

mkdir -p "$LOG_DIR/build" "$LOG_DIR/trials"
echo "build logs : $LOG_DIR/build"
echo "trial logs : $LOG_DIR/trials"

build_rc=0
if [ "$DO_BUILD" = 1 ]; then
    echo
    echo "################ building"
    bash "$BUILD_SH" --log-dir "$LOG_DIR/build" ${BUILD_ARGS[@]+"${BUILD_ARGS[@]}"}
    build_rc=$?
    if [ "$build_rc" != 0 ]; then
        echo
        echo "A build failed; the trials are not run. Read $LOG_DIR/build, fix it, and run again."
        echo "To run the trials against what did build:"
        echo "  bash $TRIALS_SH ${TRIAL_ARGS[*]:-}"
        exit "$build_rc"
    fi
fi

trial_rc=0
if [ "$DO_TRIALS" = 1 ]; then
    echo
    echo "################ trials"
    bash "$TRIALS_SH" --log-dir "$LOG_DIR/trials" ${TRIAL_ARGS[@]+"${TRIAL_ARGS[@]}"}
    trial_rc=$?
fi

echo
if [ "$build_rc" = 0 ] && [ "$trial_rc" = 0 ]; then
    echo "every build and every trial run passed; logs in $LOG_DIR"
else
    echo "builds exited $build_rc, trials exited $trial_rc; logs in $LOG_DIR"
fi
[ "$build_rc" = 0 ] && [ "$trial_rc" = 0 ]
