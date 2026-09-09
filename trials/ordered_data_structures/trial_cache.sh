#!/bin/sh
# Content-addressed compile cache for standard_data_structures leaves.
#
# Every trial that says `use wbt_set; use wbt_map;` pays ~115s to re-derive those
# modules from source, and `INTEGRATE_PRE_CLEAN` throws the result away before the
# next integrate. Nothing about that work depends on anything but the compiler and
# the source it reads, so it is cacheable.
#
# Cache key for a unit = sha256 of
#   - the silica-compiler binary (symlink resolved: a new seed invalidates everything)
#   - that unit's own source
#   - every lib/*.silica in the leaf (the whole dependency surface; trials never
#     `use` each other, only lib modules, so this covers what the unit can see)
#
# On a hit we restore the unit's .sams AND .iface. Restoring the .iface is what
# makes the hit take effect: SEED_SDS_COMPILE_ORDER (leaf.mk) and pending_units()
# (compile_per_trial.sh) both build the compile order from units that lack a
# sibling .iface, so a restored unit drops out of the order with no other change.
#
# The gate is NOT weakened. A restored .sams is still diffed against its .ascomp
# golden, still assembled, still linked, and the executable is still run and its
# output diffed against .scout. We skip re-deriving assembly we can prove is
# byte-identical; we do not skip checking it.
#
# Usage (from a leaf directory, alongside silica.config):
#   trial_cache.sh restore    # populate .sams/.iface for unchanged units
#   trial_cache.sh store      # record outputs of units compiled this run
#   trial_cache.sh key <unit.silica>
set -eu

CACHE_DIR=${TRIAL_CACHE_DIR:-.trial_cache}
MSG_PREFIX=${MSG_PREFIX:-}
SILICA_COMPILER=${SILICA_COMPILER:?SILICA_COMPILER is required}

# Disable with TRIAL_CACHE=0 to force a full recompile.
if [ "${TRIAL_CACHE:-1}" = "0" ]; then
	exit 0
fi

hash_file() {
	# shasum follows symlinks, so lib/*.silica hash their stdlib targets.
	[ -f "$1" ] || { printf 'missing\n'; return 0; }
	shasum -a 256 "$1" | cut -d' ' -f1
}

# Hash of the compiler binary plus every lib source in this leaf. Computed once
# per invocation and reused for all units, since it is the same for all of them.
compute_base_hash() {
	{
		hash_file "$SILICA_COMPILER"
		if [ -d lib ]; then
			for f in lib/*.silica; do
				[ -e "$f" ] || continue
				printf '%s %s\n' "$(basename "$f")" "$(hash_file "$f")"
			done | sort
		fi
	} | shasum -a 256 | cut -d' ' -f1
}

unit_key() {
	printf '%s %s\n' "$BASE_HASH" "$(hash_file "$1")" | shasum -a 256 | cut -d' ' -f1
}

# .trial_cache/<flattened unit path>/<key>.{sams,iface}
unit_cache_dir() {
	printf '%s/%s\n' "$CACHE_DIR" "$(printf '%s' "${1%.silica}" | tr '/' '_')"
}

# Units this leaf builds, from silica.config (libs included).
units() {
	[ -s silica.config ] || return 0
	grep -v '^[[:space:]]*$' silica.config
}

BASE_HASH=$(compute_base_hash)

case "${1:-}" in
key)
	[ $# -ge 2 ] || { echo "usage: trial_cache.sh key <unit.silica>" >&2; exit 2; }
	unit_key "$2"
	;;

restore)
	hits=0
	misses=0
	for unit in $(units); do
		base=${unit%.silica}
		# Already satisfied (e.g. stdlib objects installed from .stdlib_cache).
		[ -f "$base.iface" ] && continue
		dir=$(unit_cache_dir "$unit")
		key=$(unit_key "$unit")
		if [ -f "$dir/$key.sams" ] && [ -f "$dir/$key.iface" ]; then
			cp -f "$dir/$key.sams" "$base.sams"
			cp -f "$dir/$key.iface" "$base.iface"
			hits=$((hits + 1))
		else
			misses=$((misses + 1))
		fi
	done
	# The per-leaf runtime is emitted by the compiler, not by any one unit, so it
	# is keyed on the compiler + all lib sources alone. Without this a run where
	# every unit hits would produce no __silica_runtime.sams to assemble.
	if [ -f "$CACHE_DIR/runtime/$BASE_HASH.sams" ] && [ ! -f "__silica_runtime.sams" ]; then
		cp -f "$CACHE_DIR/runtime/$BASE_HASH.sams" "__silica_runtime.sams"
	fi
	echo "${MSG_PREFIX}compile cache: ${hits} reused, ${misses} to compile"
	;;

store)
	stored=0
	for unit in $(units); do
		base=${unit%.silica}
		[ -f "$base.sams" ] && [ -f "$base.iface" ] || continue
		dir=$(unit_cache_dir "$unit")
		key=$(unit_key "$unit")
		[ -f "$dir/$key.sams" ] && continue
		mkdir -p "$dir"
		# Drop stale keys for this unit so the cache stays bounded.
		rm -f "$dir"/*.sams "$dir"/*.iface 2>/dev/null || true
		cp -f "$base.sams" "$dir/$key.sams"
		cp -f "$base.iface" "$dir/$key.iface"
		stored=$((stored + 1))
	done
	if [ -f "__silica_runtime.sams" ] && [ ! -f "$CACHE_DIR/runtime/$BASE_HASH.sams" ]; then
		mkdir -p "$CACHE_DIR/runtime"
		rm -f "$CACHE_DIR/runtime"/*.sams 2>/dev/null || true
		cp -f "__silica_runtime.sams" "$CACHE_DIR/runtime/$BASE_HASH.sams"
	fi
	[ "$stored" -gt 0 ] && echo "${MSG_PREFIX}compile cache: ${stored} unit(s) recorded"
	exit 0
	;;

*)
	echo "usage: trial_cache.sh {restore|store|key <unit.silica>}" >&2
	exit 2
	;;
esac
