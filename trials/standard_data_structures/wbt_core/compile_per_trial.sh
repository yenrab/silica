#!/bin/sh
# Compile each trial as its own silica-compiler application: one process per
# trial with lib/*.silica + that single trial, then exit so RSS resets.
# Usage (from wbt_core/): SILICA_COMPILER=... ./compile_per_trial.sh
set -eu

MSG_PREFIX=${MSG_PREFIX:-standard_data_structures_phase1/wbt_core/}
SILICA_COMPILER=${SILICA_COMPILER:?SILICA_COMPILER is required}

if [ ! -s silica.config ]; then
	echo "SKIP: ${MSG_PREFIX}no silica.config"
	exit 0
fi

cp silica.config silica.config.full
cleanup() {
	mv -f silica.config.full silica.config 2>/dev/null || true
	rm -f .compile_trials.list .compile_libs.list
}
trap cleanup EXIT INT TERM

grep -v '^lib/' silica.config.full | grep -v '^$' > .compile_trials.list || true
grep '^lib/' silica.config.full > .compile_libs.list || true

done_count=0
fail_ec=0

while IFS= read -r trial || [ -n "$trial" ]; do
	[ -n "$trial" ] || continue
	{ cat .compile_libs.list; echo "$trial"; } > silica.config
	units=$(wc -l < silica.config | tr -d ' ')
	echo "Compiling trial ${trial} (${units} units: lib + 1 trial)..."
	rm -f silica.compile.order silica.needs_runtime
	# Seed exits 75 between multi-unit batches so the OS reclaims host heap.
	while true; do
		set +e
		"${SILICA_COMPILER}"
		ec=$?
		set -e
		if [ "$ec" -eq 0 ]; then
			break
		fi
		if [ "$ec" -eq 75 ]; then
			echo "  (reclaiming memory; continuing next unit)"
			continue
		fi
		fail_ec=$ec
		break
	done
	if [ "$fail_ec" -ne 0 ]; then
		break
	fi
	done_count=$((done_count + 1))
done < .compile_trials.list

if [ "$fail_ec" -ne 0 ]; then
	if [ "$fail_ec" -eq 137 ] || [ "$fail_ec" -eq 9 ]; then
		echo "❌❌ ${MSG_PREFIX}compilation killed (exit ${fail_ec}; likely OOM while compiling a trial + libs)"
	else
		echo "❌❌ ${MSG_PREFIX}compilation failed (exit ${fail_ec})"
	fi
	exit 1
fi

echo "Compiled ${done_count} trial(s) as separate applications"
