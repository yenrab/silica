# Copyright 2026 Lee Scott Barney
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# Root of one board trial run: trials/.target/<target>/Makefile (written by trial_target.sh)
# sets TRIAL_TARGET, BOARD_SUITES and BOARD_SKIPPED_SUITES and includes this file. Same shape as
# trials/Makefile: the shared integrate wrapper (../silica_compiler.mk) provides the progress line,
# the watchdog, the log and the report; this file fans out over the suites and sums them.
#
# Suites run concurrently (their compiles and image builds overlap); the board itself is used by
# one trial at a time (board_suite.sh takes .board.lock around every run), so while one suite is
# on the board the others compile.

_BOARD_TARGETS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
include $(_BOARD_TARGETS_DIR)../silica_compiler.mk

# error_enforcement_addition is the longest (compile-only) suite: start it first.
BOARD_ORDER := $(filter error_enforcement_addition,$(BOARD_SUITES)) $(filter-out error_enforcement_addition,$(BOARD_SUITES))
BOARD_DIR_TARGETS := $(BOARD_ORDER:%=board-dir-%)

.DEFAULT_GOAL := integrate
.PHONY: integrate-run board-dirs board-summary clean $(BOARD_DIR_TARGETS)

integrate-run:
	@rm -f "$(INTEGRATE_DIR).board.lock" "$(INTEGRATE_DIR).board_lost"
	@$(MAKE) --no-print-directory -f "$(INTEGRATE_MAKEFILE)" board-dirs || true
	@$(MAKE) --no-print-directory -f "$(INTEGRATE_MAKEFILE)" board-summary

board-dirs: $(BOARD_DIR_TARGETS)

$(BOARD_DIR_TARGETS): board-dir-%:
	@$(call INTEGRATE_CHILD,$*)

# Per-suite lines (skipped suites with their reason, run suites with their counts and the number
# of trials the skip list or the driver left out), then the tree totals in .integrate_counts.
board-summary:
	@cd "$(INTEGRATE_DIR)" && ok=0; ko=0; failed=0; \
	printf '──── %s: per-directory results ────\n' "$(INTEGRATE_LABEL)"; \
	for dir in $(BOARD_ORDER); do \
		n1=0; n2=0; \
		if [ -f "$$dir/.integrate_counts" ]; then read n1 n2 < "$$dir/.integrate_counts" || true; fi; \
		n1=$${n1:-0}; n2=$${n2:-0}; \
		cst=$$(cat "$$dir/.integrate_status" 2>/dev/null || echo 1); \
		if [ "$$cst" -ne 0 ] && [ "$$n2" -eq 0 ]; then n2=1; fi; \
		el=$$(sed -n 's/^elapsed: //p' "$$dir/.integrate_report" 2>/dev/null | head -1); \
		sk=$$(cat "$$dir/.skipped_count" 2>/dev/null || echo 0); \
		extra=""; [ "$$sk" -gt 0 ] && extra="  (skipped $$sk)"; \
		printf '  %-48s ✅✅ %6s  ❌❌ %6s  %s%s\n' "$$dir" "$$n1" "$$n2" "$$el" "$$extra"; \
		ok=$$((ok + n1)); ko=$$((ko + n2)); \
		[ "$$n2" -ne 0 ] && failed=1; \
	done; \
	while IFS='	' read -r dir reason; do \
		[ -n "$$dir" ] || continue; \
		printf '  %-48s SKIPPED (%s)\n' "$$dir" "$$reason"; \
	done < .skipped_suites; \
	if [ -f .board_lost ]; then printf '  board lost during the run: %s\n' "$$(cat .board_lost)"; fi; \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	[ "$$ko" -eq 0 ] && [ "$$failed" -eq 0 ]

clean:
	@for dir in $(BOARD_SUITES); do rm -rf "$(INTEGRATE_DIR)$$dir/src" "$(INTEGRATE_DIR)$$dir/build"; done
