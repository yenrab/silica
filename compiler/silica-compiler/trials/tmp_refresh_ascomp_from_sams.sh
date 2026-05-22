#!/usr/bin/env bash
# Copy trial assembly goldens: for every *.sams under ROOT, overwrite the sibling *.ascomp
# if it exists (same basename, same directory; find descends arbitrarily deep).
#
# Usage:
#   ./tmp_refresh_ascomp_from_sams.sh [ROOT]
#   SILICA_TRIALS_ROOT=/path/to/trials ./tmp_refresh_ascomp_from_sams.sh
#
# Default ROOT resolves to the silica-compiler *trials bundle* directory (the directory that
# contains structs_addition, supervisors_addition, traits_addition, tuples_addition, …)
# even if this script moves under a subdirectory of trials, instead of silently defaulting to
# that narrow subtree only.
#
# Explicit overrides:
#   1) positional ROOT
#   2) env SILICA_TRIALS_ROOT when no positional ROOT
#
# Old “only cwd” behavior: ./tmp_refresh_ascomp_from_sams.sh .
#
# One-line progress: set VERBOSE=1
#
# Pairing rule: for path/to/Base.sams we only overwrite path/to/Base.ascomp when that .ascomp
# already exists beside the .sams. Missing .ascomp is a silent skip for that .sams (not an error).

set -euo pipefail

script_here="${BASH_SOURCE[0]}"
while [[ -L "$script_here" ]]; do
	script_here="$(readlink "$script_here")"
done

start_dir="$(CDPATH="" cd "$(dirname -- "$script_here")" && pwd -P)"

is_silica_compiler_trials_dir() {
	local d="$1"
	[[ "$(basename "$d")" == "trials" ]] || return 1
	[[ -f "$d/../src/silica-compiler" ]] || [[ -x "$d/../src/silica-compiler" ]] || return 1
	return 0
}

# Walk up from the script (e.g. .../trials/foo/bar) until we reach .../silica-compiler/trials.
default_root=""
d="$start_dir"
for ((i = 0; i < 40; i++)); do
	if is_silica_compiler_trials_dir "$d"; then
		default_root="$d"
		break
	fi
	if [[ "$d" == "/" ]]; then
		break
	fi
	d="$(dirname "$d")"
done
default_root="${default_root:-$start_dir}"

raw_root="${1:-${SILICA_TRIALS_ROOT:-$default_root}}"
if [[ ! -d "$raw_root" ]]; then
	printf 'not a directory: %s\n' "$raw_root" >&2
	exit 1
fi

root="$(CDPATH="" cd "$raw_root" && pwd -P)"
updated=0
skipped_no_ascomp=0

[[ -n "${VERBOSE:-}" ]] && printf 'tmp_refresh_ascomp_from_sams: root=%s\n' "$root" >&2

# -L: follow symlinked directories when descending
while IFS= read -r -d '' sams; do
	dir="$(dirname -- "$sams")"
	base="$(basename -- "$sams" .sams)"
	ascomp="${dir}/${base}.ascomp"
	if [[ ! -f "$ascomp" ]]; then
		skipped_no_ascomp=$((skipped_no_ascomp + 1))
		[[ -n "${VERBOSE:-}" ]] && printf 'skip (no sibling .ascomp): %s\n' "$sams" >&2
		continue
	fi
	cp "$sams" "$ascomp"
	printf 'updated %s\n' "$ascomp"
	updated=$((updated + 1))
done < <(find -L "$root" -type f -name '*.sams' -print0)

printf 'updated %d .ascomp file(s)\n' "$updated"
if [[ "$skipped_no_ascomp" -gt 0 ]]; then
	printf 'note: skipped %d .sams (need Base.ascomp beside Base.sams; VERBOSE=1 lists each)\n' \
		"$skipped_no_ascomp" >&2
fi
