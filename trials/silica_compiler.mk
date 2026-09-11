# Shared seed host for trials — always binaries/silica-compiler unless overridden
# on the make command line (e.g. `make SILICA_COMPILER=/path/to/other integrate`).
# Paths are anchored to this file (trials/silica_compiler.mk).
#
# Usage (from any trials Makefile):
#   THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
#   include $(THIS_DIR)../silica_compiler.mk          # depth-1 trial
#   include $(THIS_DIR)../../silica_compiler.mk        # depth-2 trial
#   include silica_compiler.mk                        # trials/Makefile
#
# IMPORTANT: capture THIS_DIR (or MAKEFILE_DIR / LEAF_DIR) *before* this include.
# After include, $(lastword $(MAKEFILE_LIST)) is this file (trials/), so recipes that
# still use MAKEFILE_LIST write silica.config in the wrong directory and integrate SKIPs.
#
# Leaf helpers that already include this file (do not re-point the compiler elsewhere):
#   standard_data_structures_phase1/leaf.mk
#   ffi_addition/common_app.mk

_SILICA_COMPILER_MK_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
BINARIES_DIR := $(abspath $(_SILICA_COMPILER_MK_DIR)../binaries)
_SILICA_COMPILER_SEED := $(BINARIES_DIR)/silica-compiler

# Force the seed binary for undefined / environment origins. Command-line
# SILICA_COMPILER=... still wins (GNU make: origin "command line").
ifeq ($(filter command line,$(origin SILICA_COMPILER)),)
SILICA_COMPILER := $(_SILICA_COMPILER_SEED)
endif

UPDATE_SILICA_COMPILER_LINK := $(BINARIES_DIR)/update_silica_compiler_link.bash

# If silica-compiler is missing, run update_silica_compiler_link.bash and re-check.
define ENSURE_SILICA_COMPILER
	if [ ! -x "$(SILICA_COMPILER)" ]; then \
		echo "Missing $(SILICA_COMPILER); running update_silica_compiler_link.bash..."; \
		"$(UPDATE_SILICA_COMPILER_LINK)" || exit 1; \
		if [ ! -x "$(SILICA_COMPILER)" ]; then \
			echo "Missing seed host after update: $(SILICA_COMPILER)"; \
			exit 1; \
		fi; \
	fi
endef

.PHONY: ensure-silica-compiler
ensure-silica-compiler:
	@$(ENSURE_SILICA_COMPILER)

# Merged into every including Makefile's compile/assembly when present.
# (`integrate` is defined by the wrapper section at the end of this file.)
compile: ensure-silica-compiler
assembly: ensure-silica-compiler

# Merged into every including Makefile's clean: drop published unit iface caches under
# that trial's CURDIR (make -C <trial> clean). Nested *.iface are included. Also drops
# the integrate wrapper's bookkeeping files (see the wrapper section below).
.PHONY: clean-silica-ifaces
clean: clean-silica-ifaces
clean-silica-ifaces:
	@find "$(CURDIR)" -name '*.iface' -type f -delete 2>/dev/null || true
	@[ -n "$$SILICA_INTEGRATE_ACTIVE" ] || { rm -f $(INTEGRATE_FILES); rm -rf "$(INTEGRATE_RUNNING_DIR)"; }

# So prerequisites like `integrate: $(SILICA_COMPILER)` create the binaries/ link if needed.
$(SILICA_COMPILER):
	@$(ENSURE_SILICA_COMPILER)

# Multi-unit batches use process-per-unit hygiene: the seed exits 75 (EX_TEMPFAIL)
# between units so the OS reclaims host heap. Recipes must re-invoke until exit 0
# (or a hard failure). Usage inside a recipe (same cwd as silica.config):
#   $(RUN_SILICA_COMPILER)
# Optional override of the binary path:
#   $(call RUN_SILICA_COMPILER_WITH,$(abspath $(SILICA_COMPILER)))
# Quiet variant (no reclaim chatter) for golden-output capture:
#   $(call RUN_SILICA_COMPILER_QUIET_WITH,"$(SILICA_COMPILER)")
# Per-unit compile timeout. Without it a compiler hang blocks the suite indefinitely --
# an orphaned resume loop once spun for 14 hours overnight. Exit 142 = alarm fired.
SILICA_COMPILE_TIMEOUT ?= 300
define RUN_SILICA_COMPILER_WITH
	while true; do \
		perl -e 'alarm shift @ARGV; exec @ARGV' $(SILICA_COMPILE_TIMEOUT) $(1); \
		ec=$$?; \
		if [ $$ec -eq 0 ]; then break; fi; \
		if [ $$ec -eq 75 ]; then \
			echo "  (reclaiming memory; continuing next unit)"; \
			continue; \
		fi; \
		if [ $$ec -eq 142 ]; then \
			echo "❌❌ compiler TIMED OUT after $(SILICA_COMPILE_TIMEOUT)s (compile hang)"; \
		fi; \
		exit $$ec; \
	done
endef

# Quiet reclaim loop for golden capture. Do not `exit` on hard failure: recipe lines that
# redirect into `.cur_fail` often run in the make shell (not a subshell), and `exit`
# would abort the recipe before the diff step. Callers use `|| true` around the loop.
define RUN_SILICA_COMPILER_QUIET_WITH
	while true; do \
		perl -e 'alarm shift @ARGV; exec @ARGV' $(SILICA_COMPILE_TIMEOUT) $(1); \
		ec=$$?; \
		if [ $$ec -eq 0 ]; then break; fi; \
		if [ $$ec -eq 75 ]; then continue; fi; \
		break; \
	done
endef

define RUN_SILICA_COMPILER
	$(call RUN_SILICA_COMPILER_WITH,"$(SILICA_COMPILER)")
endef

define RUN_SILICA_COMPILER_QUIET
	$(call RUN_SILICA_COMPILER_QUIET_WITH,"$(SILICA_COMPILER)")
endef

# ============================================================================
# integrate wrapper -- parallel fan-out, per-directory log and report
# ============================================================================
#
# Every trial Makefile names its real integrate recipe `integrate-run`. The shared
# `integrate` target below wraps it:
#
#   1. records the start time,
#   2. runs `integrate-run` (parallel when no jobserver is already active),
#      capturing everything it prints to <dir>/.integrate_log,
#   3. writes a report for just this directory to <dir>/.integrate_report: pass/fail
#      totals, elapsed time, every failure with the file and the kind of difference,
#      and the log path. The outermost directory prints its report; nested ones only
#      write theirs (the outermost failure list replays every nested failure).
#
# Parents that fan out over children use $(call INTEGRATE_CHILD,<subdir>[,flags]):
# the child's output goes only to its own log; the terminal shows one dot per check
# (see the progress channel below) and the outermost report at the end. Diff bodies
# stay in the child's log and are replayed, deduplicated, in the failure section.
#
# Counts are the existing per-directory `.integrate_counts` files ("<ok> <fail>").
# Nothing here parses free-form output for totals.
#
# Knobs (command line or environment):
#   JOBS=N                 parallelism for a standalone `make integrate` (default: cores)
#   SDS_JOBS=N             parallelism for ordered_data_structures (memory-bound, default 4)
#   INTEGRATE_DETAIL_LINES per-failure context lines replayed in reports (default 40)
#   INTEGRATE_WATCHDOG_MINUTES minutes of silence before running trials are killed (default 15)

INTEGRATE_MAKEFILE := $(abspath $(firstword $(MAKEFILE_LIST)))
INTEGRATE_DIR      := $(dir $(INTEGRATE_MAKEFILE))
_INTEGRATE_TRIALS_ROOT := $(patsubst %/,%,$(abspath $(_SILICA_COMPILER_MK_DIR)))
_INTEGRATE_DIR_NOSLASH := $(patsubst %/,%,$(INTEGRATE_DIR))
INTEGRATE_LABEL := $(if $(filter $(_INTEGRATE_TRIALS_ROOT),$(_INTEGRATE_DIR_NOSLASH)),trials,$(patsubst $(_INTEGRATE_TRIALS_ROOT)/%,%,$(_INTEGRATE_DIR_NOSLASH)))

INTEGRATE_FILES := $(INTEGRATE_DIR).integrate_log $(INTEGRATE_DIR).integrate_report \
	$(INTEGRATE_DIR).integrate_status $(INTEGRATE_DIR).integrate_start \
	$(INTEGRATE_DIR).integrate_results $(INTEGRATE_DIR).integrate_skipped \
	$(INTEGRATE_DIR).integrate_last_dot
INTEGRATE_RUNNING_DIR := $(INTEGRATE_DIR).integrate_running

ifeq ($(origin JOBS),undefined)
JOBS := $(shell sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4)
endif
export JOBS
SDS_JOBS ?= 4
export SDS_JOBS
INTEGRATE_DETAIL_LINES ?= 40
export INTEGRATE_DETAIL_LINES
# The watchdog kills every registered running trial after this many minutes without a dot.
INTEGRATE_WATCHDOG_MINUTES ?= 15
export INTEGRATE_WATCHDOG_MINUTES

# A Makefile may set INTEGRATE_JOBS before or after including this file (leaf.mk and
# ordered_data_structures use SDS_JOBS). When make already runs under a jobserver the
# sub-make inherits it and no -j is added.
INTEGRATE_JOBS ?= $(JOBS)
INTEGRATE_JOBFLAGS = $(if $(filter -j% --jobserver%,$(MAKEFLAGS)),,-j$(INTEGRATE_JOBS))

# ---- progress channel ---------------------------------------------------------------
# The outermost `integrate` opens file descriptor 9 on its terminal and exports
# SILICA_INTEGRATE_ROOT (its directory), SILICA_INTEGRATE_DOT_OK and SILICA_INTEGRATE_DOT_FAIL
# (a full-block glyph U+2588 in the foreground colour when fd 9 is a terminal: blue (256-colour
# index 33) for a pass, orange (208) for a failure -- a pair that stays distinct under red-green
# and blue-yellow colour blindness; the background attribute is never set -- `.` and `F`
# otherwise). Every check prints
# one dot to fd 9 with a single write, so parallel suites never tear the line, and touches
# the root's .integrate_last_dot for the watchdog. Nothing else reaches the terminal until
# the root prints its report. Without the wrapper (fd 9 closed) the dots are silently dropped.
# fd 3 and fd 4 are the make 3.81 jobserver pipe; never use them for this.
INTEGRATE_DOT_OK   = { printf '%s' "$$SILICA_INTEGRATE_DOT_OK" >&9; touch "$$SILICA_INTEGRATE_ROOT/.integrate_last_dot"; } 2>/dev/null || true
INTEGRATE_DOT_FAIL = { printf '%s' "$$SILICA_INTEGRATE_DOT_FAIL" >&9; touch "$$SILICA_INTEGRATE_ROOT/.integrate_last_dot"; } 2>/dev/null || true

# ---- trial runs -----------------------------------------------------------------------
# Every trial executable (and every helper script that drives one) runs through one of
# these so the watchdog can find and kill it: the process runs in its own process group,
# registered in the root's .integrate_running/ while it runs. Afterwards $$integrate_rc is
# its exit status and $$integrate_killed is 1 when the watchdog killed it, in which case
# the recipe reports the trial as incomplete instead of comparing its output.
#
# $(call INTEGRATE_RUN_TRIAL,<label>,<sout path>,<command>): output then exit status into <sout path>.
define INTEGRATE_RUN_TRIAL
integrate_killed=0; \
perl -e 'setpgrp(0, 0); exec @ARGV or exit 127' $(3) > "$(2)" 2>&1 & integrate_pid=$$!; \
integrate_reg="$${SILICA_INTEGRATE_ROOT:-.}/.integrate_running/$$integrate_pid"; \
{ printf '%s %s\n' "$$integrate_pid" "$(1)" > "$$integrate_reg"; } 2>/dev/null; \
wait $$integrate_pid; integrate_rc=$$?; \
printf '%s\n' "$$integrate_rc" >> "$(2)"; \
if [ -f "$$integrate_reg.killed" ]; then integrate_killed=1; fi; \
rm -f "$$integrate_reg" "$$integrate_reg.killed" 2>/dev/null
endef
# $(call INTEGRATE_EXEC_TRIAL,<label>,<command>): the command manages its own output.
define INTEGRATE_EXEC_TRIAL
integrate_killed=0; \
perl -e 'setpgrp(0, 0); exec @ARGV or exit 127' $(2) & integrate_pid=$$!; \
integrate_reg="$${SILICA_INTEGRATE_ROOT:-.}/.integrate_running/$$integrate_pid"; \
{ printf '%s %s\n' "$$integrate_pid" "$(1)" > "$$integrate_reg"; } 2>/dev/null; \
wait $$integrate_pid; integrate_rc=$$?; \
if [ -f "$$integrate_reg.killed" ]; then integrate_killed=1; fi; \
rm -f "$$integrate_reg" "$$integrate_reg.killed" 2>/dev/null
endef
# The message a recipe prints for a trial the watchdog killed.
INTEGRATE_KILLED_MSG = killed by the watchdog after $$SILICA_INTEGRATE_WATCHDOG_MINUTES minutes of silence; trial incomplete

.PHONY: integrate integrate-run integrate-report

# SILICA_INTEGRATE_ACTIVE tells the shared clean hook not to delete the log and start
# stamp while the run they belong to is in progress (most integrate-run recipes clean first).
#
# Recipe lines that contain $(MAKE) are executed even under `make -n` (make passes -n
# down instead), so every such line here and in the fan-out Makefiles carries nothing
# but the sub-make and its output plumbing; file writes sit on their own lines. Under
# -n the wrapper only forwards the dry run to integrate-run and writes nothing.
integrate: ensure-silica-compiler
ifneq (,$(findstring n,$(firstword -$(MAKEFLAGS))))
	@$(MAKE) --no-print-directory -C "$(INTEGRATE_DIR)" -f "$(INTEGRATE_MAKEFILE)" integrate-run
else
	@date +%s > "$(INTEGRATE_DIR).integrate_start"; \
	rm -f "$(INTEGRATE_DIR).integrate_log" "$(INTEGRATE_DIR).integrate_status" "$(INTEGRATE_DIR).integrate_report"
	@if [ -n "$$SILICA_INTEGRATE_ROOT" ]; then \
		export SILICA_INTEGRATE_ACTIVE=1; \
		{ $(MAKE) --no-print-directory $(INTEGRATE_JOBFLAGS) -C "$(INTEGRATE_DIR)" -f "$(INTEGRATE_MAKEFILE)" integrate-run; \
		  echo $$? > "$(INTEGRATE_DIR).integrate_status"; } 2>&1 | tee -a "$(INTEGRATE_DIR).integrate_log"; \
		$(MAKE) --no-print-directory -C "$(INTEGRATE_DIR)" -f "$(INTEGRATE_MAKEFILE)" integrate-report; \
	else \
		exec 9>&1; \
		export SILICA_INTEGRATE_ACTIVE=1 SILICA_INTEGRATE_ROOT="$(_INTEGRATE_DIR_NOSLASH)" SILICA_INTEGRATE_WATCHDOG_MINUTES="$(INTEGRATE_WATCHDOG_MINUTES)"; \
		if [ -t 9 ]; then export SILICA_INTEGRATE_DOT_OK="$$(printf '\033[38;5;33m\342\226\210\033[0m')" SILICA_INTEGRATE_DOT_FAIL="$$(printf '\033[38;5;208m\342\226\210\033[0m')"; \
		else export SILICA_INTEGRATE_DOT_OK="." SILICA_INTEGRATE_DOT_FAIL="F"; fi; \
		rm -rf "$(INTEGRATE_RUNNING_DIR)"; mkdir -p "$(INTEGRATE_RUNNING_DIR)"; touch "$(INTEGRATE_DIR).integrate_last_dot"; \
		( wd_secs=$$(( $(INTEGRATE_WATCHDOG_MINUTES) * 60 )); \
		  while sleep 30; do \
			last=$$(stat -f %m "$(INTEGRATE_DIR).integrate_last_dot" 2>/dev/null || echo 0); now=$$(date +%s); \
			if [ $$((now - last)) -ge "$$wd_secs" ]; then \
				for reg in "$(INTEGRATE_RUNNING_DIR)"/*; do \
					[ -f "$$reg" ] || continue; case "$$reg" in *.killed) continue;; esac; \
					read -r pid label < "$$reg"; : > "$$reg.killed"; \
					kill -TERM -- "-$$pid" 2>/dev/null; sleep 5; kill -KILL -- "-$$pid" 2>/dev/null; \
				done; \
				touch "$(INTEGRATE_DIR).integrate_last_dot"; \
			fi; \
		  done ) & wd_pid=$$!; \
		{ $(MAKE) --no-print-directory $(INTEGRATE_JOBFLAGS) -C "$(INTEGRATE_DIR)" -f "$(INTEGRATE_MAKEFILE)" integrate-run; \
		  echo $$? > "$(INTEGRATE_DIR).integrate_status"; } 2>&1 | tee -a "$(INTEGRATE_DIR).integrate_log" > /dev/null; \
		kill "$$wd_pid" 2>/dev/null; wait "$$wd_pid" 2>/dev/null; \
		rm -rf "$(INTEGRATE_RUNNING_DIR)"; \
		printf '\n' >&9; \
		$(MAKE) --no-print-directory -C "$(INTEGRATE_DIR)" -f "$(INTEGRATE_MAKEFILE)" integrate-report; \
	fi
endif

integrate-report:
	@$(INTEGRATE_REPORT)

# Failure replay: every ❌ line in every .integrate_log under this directory, each
# followed by the context the recipe printed after it (the diff), capped at
# INTEGRATE_DETAIL_LINES lines. Deepest logs first, so the copy with the diff wins
# and the bare copies echoed by enclosing directories are dropped as duplicates.
define INTEGRATE_FAILURE_DETAILS
logs=$$(find "$(INTEGRATE_DIR)" -name .integrate_log -print 2>/dev/null | awk '{ n = gsub("/", "/"); print n "\t" $$0 }' | sort -rn | cut -f2-); \
[ -n "$$logs" ] && awk -v cap="$(INTEGRATE_DETAIL_LINES)" ' \
	function flush() { if (blk != "") print blk; blk = "" } \
	FNR == 1 { flush(); skip = 1 } \
	/^(❌❌|❌ )|^ (❌❌|❌ )|^  (❌❌|❌ )/ { flush(); if ($$0 in seen) { skip = 1; next } seen[$$0] = 1; blk = $$0; n = 0; skip = 0; next } \
	/^(✅✅|✅ )|^ (✅✅|✅ )|^  (✅✅|✅ )|^(=== |SKIP:|Assembling|Linking|Running|Compiling|Reusing|make\[|════|success:|fail:|elapsed:|log:|failures:)/ { flush(); skip = 1; next } \
	{ if (!skip) { if (n < cap) blk = blk "\n" $$0; else if (n == cap) blk = blk "\n(... more in log)"; n++ } } \
	END { flush() }' $$logs
endef

define INTEGRATE_REPORT
cd "$(INTEGRATE_DIR)" || exit 1; \
st=$$(cat .integrate_status 2>/dev/null || echo 1); \
ok=0; ko=0; \
if [ -f .integrate_counts ]; then read ok ko < .integrate_counts || true; fi; \
ok=$${ok:-0}; ko=$${ko:-0}; \
if [ "$$st" -ne 0 ] && [ "$$ko" -eq 0 ]; then ko=1; fi; \
start=$$(cat .integrate_start 2>/dev/null || date +%s); now=$$(date +%s); el=$$((now - start)); \
if [ "$$el" -ge 3600 ]; then elapsed=$$(printf '%dh %02dm %02ds' $$((el / 3600)) $$((el % 3600 / 60)) $$((el % 60))); \
elif [ "$$el" -ge 60 ]; then elapsed=$$(printf '%dm %02ds' $$((el / 60)) $$((el % 60))); \
else elapsed=$$(printf '%ds' "$$el"); fi; \
{ \
	printf '════ integrate report: %s ════\n' "$(INTEGRATE_LABEL)"; \
	printf 'success: ✅✅ %s\n' "$$ok"; \
	printf 'fail: ❌❌ %s\n' "$$ko"; \
	printf 'elapsed: %s\n' "$$elapsed"; \
	if [ "$$ko" -ne 0 ]; then \
		printf 'failures:\n'; \
		details=$$({ $(INTEGRATE_FAILURE_DETAILS); }); \
		if [ -n "$$details" ]; then printf '%s\n' "$$details" | sed 's/^/    /'; \
		else printf '    (no ❌ line was printed; the run ended with status %s -- last %s lines of the log)\n' "$$st" "$(INTEGRATE_DETAIL_LINES)"; \
			tail -n "$(INTEGRATE_DETAIL_LINES)" .integrate_log 2>/dev/null | sed 's/^/    /'; fi; \
	fi; \
	printf 'log: %s\n' "$(INTEGRATE_DIR).integrate_log"; \
} > .integrate_report; \
if [ "$$SILICA_INTEGRATE_ROOT" = "$(_INTEGRATE_DIR_NOSLASH)" ]; then cat .integrate_report; fi; \
[ "$$ko" -eq 0 ] && [ "$$st" -eq 0 ]
endef

# $(call INTEGRATE_CHILD,<subdir relative to INTEGRATE_DIR>[,extra make args])
# Never fails the calling recipe: the parent sums the children's .integrate_counts
# afterwards and decides the exit status itself, so every sibling always runs.
define INTEGRATE_CHILD
cd "$(INTEGRATE_DIR)" || exit 1; \
dir="$(1)"; \
rm -f "$$dir/.integrate_counts" "$$dir/.integrate_report" "$$dir/.integrate_status" "$$dir/.integrate_skipped"; \
if [ -f "$$dir/INTEGRATE_PENDING" ]; then \
	echo "SKIP: $$dir (INTEGRATE_PENDING -- see $$dir/README.md)"; \
	: > "$$dir/.integrate_skipped"; \
else \
	$(MAKE) --no-print-directory -C "$$dir" integrate $(2) > /dev/null 2>&1; \
	if [ ! -f "$$dir/.integrate_report" ]; then \
		echo "❌❌ $$dir: integrate produced no report (make failed before reporting; see $$dir/.integrate_log)"; \
		$(INTEGRATE_DOT_FAIL); \
	fi; \
fi
endef

# Sum the .integrate_counts of listed child directories (skipped children are not
# counted; a child with no counts file and a non-zero status counts as one failure).
# $(call INTEGRATE_SUM_CHILDREN,<dirs>) -- leaves $ok / $ko / $failed set in the shell.
define INTEGRATE_SUM_CHILDREN
ok=0; ko=0; failed=0; \
printf '──── %s: per-directory results ────\n' "$(INTEGRATE_LABEL)"; \
for dir in $(1); do \
	if [ -f "$$dir/.integrate_skipped" ]; then \
		printf '  %-48s SKIPPED (INTEGRATE_PENDING)\n' "$$dir"; \
		continue; \
	fi; \
	n1=0; n2=0; \
	if [ -f "$$dir/.integrate_counts" ]; then read n1 n2 < "$$dir/.integrate_counts" || true; fi; \
	n1=$${n1:-0}; n2=$${n2:-0}; \
	cst=$$(cat "$$dir/.integrate_status" 2>/dev/null || echo 1); \
	if [ "$$cst" -ne 0 ] && [ "$$n2" -eq 0 ]; then n2=1; fi; \
	el=$$(sed -n 's/^elapsed: //p' "$$dir/.integrate_report" 2>/dev/null | head -1); \
	printf '  %-48s ✅✅ %6s  ❌❌ %6s  %s\n' "$$dir" "$$n1" "$$n2" "$$el"; \
	ok=$$((ok + n1)); ko=$$((ko + n2)); \
	[ "$$n2" -ne 0 ] && failed=1; \
done; \
printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts
endef
