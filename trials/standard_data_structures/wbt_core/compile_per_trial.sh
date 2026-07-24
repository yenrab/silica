#!/bin/sh
# Compile each trial as its own silica-compiler application: one process per
# trial with lib/*.silica + that single trial, then exit so RSS resets.
#
# The compiler prints "Files to compile: N" / "units left: N" and exits 75
# between units. That counter restarts at (lib count + 1) for EVERY trial —
# that is expected progress, not a stuck loop. Watch the "trial i/N" lines.
#
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

total_trials=$(grep -c '.' .compile_trials.list 2>/dev/null || echo 0)
lib_units=$(grep -c '.' .compile_libs.list 2>/dev/null || echo 0)
units_per_trial=$((lib_units + 1))

echo "${MSG_PREFIX}per-trial compile: ${total_trials} trial(s) × ${units_per_trial} units (libs+trial); unit counter resets each trial"

done_count=0
fail_ec=0
trial_index=0

while IFS= read -r trial || [ -n "$trial" ]; do
	[ -n "$trial" ] || continue
	trial_index=$((trial_index + 1))
	{ cat .compile_libs.list; echo "$trial"; } > silica.config
	echo "${MSG_PREFIX}trial ${trial_index}/${total_trials}: ${trial} (${units_per_trial} units)..."
	rm -f silica.compile.order silica.needs_runtime
	prev_left=-1
	stalls=0
	# Seed exits 75 between multi-unit batches so the OS reclaims host heap.
	while true; do
		set +e
		"${SILICA_COMPILER}"
		ec=$?
		set -e
		if [ "$ec" -eq 0 ]; then
			echo "${MSG_PREFIX}trial ${trial_index}/${total_trials}: ${trial} done"
			break
		fi
		if [ "$ec" -eq 75 ]; then
			if [ -f silica.compile.order ]; then
				left=$(grep -c '.' silica.compile.order 2>/dev/null || echo 0)
			else
				left=$units_per_trial
			fi
			echo "  ${MSG_PREFIX}reclaim; units left in this trial: ${left}/${units_per_trial}"
			if [ "$prev_left" -ge 0 ] && [ "$left" -ge "$prev_left" ]; then
				stalls=$((stalls + 1))
				if [ "$stalls" -ge 3 ]; then
					echo "❌❌ ${MSG_PREFIX}compile order not shrinking for ${trial} (stuck at ${left} units)"
					fail_ec=75
					break
				fi
			else
				stalls=0
			fi
			prev_left=$left
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
		echo "❌❌ ${MSG_PREFIX}compilation failed (exit ${fail_ec}) on trial ${trial_index}/${total_trials}: ${trial:-?}"
	fi
	exit 1
fi

echo "${MSG_PREFIX}Compiled ${done_count}/${total_trials} trial(s) as separate applications"
