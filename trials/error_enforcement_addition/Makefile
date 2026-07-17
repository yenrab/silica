# Makefile for Silica error enforcement trials
# Files to test: all *.silica under this directory except paths inside SUB_TRIALS
# (those multi-file trials use their own Makefile + silica.config).
# Each single-file trial is compiled in isolation inside a temporary directory: a symlink to
# the .silica file and a single-entry silica.config are created there, the
# compiler is run, then the temp directory is removed.
# Compiler output is captured to *.cur_fail and diffed against *.golden_fail.
# If a source path has a sidecar "<same-stem>.no_golden_fail" (empty marker next to
# "<stem>.silica"), that file is skipped by the single-file loop (no .golden_fail required).
# Reports ✅✅ on match, ❌❌ on mismatch or missing .golden_fail.
# Exits with failure if any check fails.
#
# Run from this directory: cd error_enforcement_addition && make

THIS_DIR        := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
include $(dir $(abspath $(lastword $(MAKEFILE_LIST))))../silica_compiler.mk

TRIAL_DIR := $(notdir $(patsubst %/,%,$(THIS_DIR)))
MSG_PREFIX := $(TRIAL_DIR)/

# Multi-file trials (checked-in silica.config + subdir Makefile).
SUB_TRIALS      := ffi_addition module_nonexported_callee_trial qualified_call_module_name_case_mismatch_trial supervisor_agent_type_invalid_atom_trial

# Single-file trial sources only (paths under SUB_TRIALS excluded; keep prune list in sync with SUB_TRIALS).
SILICA_FILES := $(shell cd "$(THIS_DIR)" && find . \( -path './ffi_addition/*' -o -path './module_nonexported_callee_trial/*' -o -path './qualified_call_module_name_case_mismatch_trial/*' -o -path './supervisor_agent_type_invalid_atom_trial/*' \) -prune -o -type f -name '*.silica' -print 2>/dev/null | sed 's|^\./||')

.DEFAULT_GOAL := all
.PHONY: all integrate clean help silica.config

silica.config: | clean
	@cd "$(dir $(abspath $(lastword $(MAKEFILE_LIST))))" && find . \( -path './ffi_addition/*' -o -path './module_nonexported_callee_trial/*' -o -path './qualified_call_module_name_case_mismatch_trial/*' -o -path './supervisor_agent_type_invalid_atom_trial/*' \) -prune -o -type f -name '*.silica' -print 2>/dev/null | sed 's|^\./||' > silica.config

# Same checks as top-level trials Makefile for this directory (error output vs .golden_fail).
integrate: clean silica.config all

all: | clean
	@failed=0; ok=0; ko=0; rm -f "$(THIS_DIR).integrate_counts"; \
	for silica in $(SILICA_FILES); do \
		[ -f "$(THIS_DIR)$$silica" ] || { echo "❌❌ $(MSG_PREFIX)$$silica: file not found"; ko=$$((ko + 1)); failed=1; continue; }; \
		base=$${silica%.silica}; \
		if [ -f "$(THIS_DIR)$$base.no_golden_fail" ]; then \
			continue; \
		fi; \
		if [ ! -f "$(THIS_DIR)$$base.golden_fail" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$base: no matching .golden_fail file"; \
			ko=$$((ko + 1)); failed=1; \
			continue; \
		fi; \
		tmpdir=$$(mktemp -d); \
		mkdir -p "$$tmpdir/$$(dirname "$$silica")"; \
		ln -s "$(THIS_DIR)$$silica" "$$tmpdir/$$silica"; \
		echo "$$silica" > "$$tmpdir/silica.config"; \
		(cd "$$tmpdir" && $(SILICA_COMPILER)) > "$(THIS_DIR)$$base.cur_fail" 2>&1 || true; \
		rm -rf "$$tmpdir"; \
		if diff -q "$(THIS_DIR)$$base.cur_fail" "$(THIS_DIR)$$base.golden_fail" > /dev/null 2>&1; then \
			echo "✅✅ $(MSG_PREFIX)$$base"; \
			ok=$$((ok + 1)); \
		else \
			echo "❌❌ $(MSG_PREFIX)$$base: .cur_fail differs from .golden_fail"; \
			diff "$(THIS_DIR)$$base.cur_fail" "$(THIS_DIR)$$base.golden_fail" || true; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
	done; \
	for sub in $(SUB_TRIALS); do \
		if $(MAKE) -C "$(THIS_DIR)$$sub" integrate COMPILER="$(SILICA_COMPILER)"; then \
			:; \
		else \
			failed=1; \
		fi; \
		if [ -f "$(THIS_DIR)$$sub/.integrate_counts" ]; then \
			read s1 s2 < "$(THIS_DIR)$$sub/.integrate_counts" || true; \
			ok=$$((ok + $${s1:-0})); ko=$$((ko + $${s2:-0})); \
		fi; \
	done; \
	[ "$$failed" -ne 0 ] && [ "$$ko" -eq 0 ] && ko=$$((ko + 1)); \
	printf '%d %d\n' "$$ok" "$$ko" > "$(THIS_DIR).integrate_counts"; \
	exit $$failed

clean:
	@rm -f $(THIS_DIR)*.cur_fail "$(THIS_DIR)silica.config" "$(THIS_DIR).integrate_counts"
	@for sub in $(SUB_TRIALS); do $(MAKE) -C "$(THIS_DIR)$$sub" clean; done
	@echo "✅ $(MSG_PREFIX)Clean complete"

help:
	@echo "Makefile - Silica error enforcement trials"
	@echo "=========================================="
	@echo "Single-file list: *.silica here excluding SUB_TRIALS ($(SUB_TRIALS)); then each subdir integrate."
	@echo "Each file is compiled in an isolated temp directory; a one-line silica.config"
	@echo "is left untouched.  Compiler output is captured to .cur_fail and"
	@echo "compared to .golden_fail.  Files with a sidecar .no_golden_fail are skipped."
	@echo ""
	@echo "Targets:"
	@echo "  all       - Single-file loop (excluding SUB_TRIALS), then make integrate in each SUB_TRIAL"
	@echo "              ✅✅ = output matches .golden_fail"
	@echo "              ❌❌ = output differs, file not found, or missing .golden_fail (when not skipped)"
	@echo "  integrate - Same as all (integration test for this directory)"
	@echo "  clean     - Remove .cur_fail files, silica.config, and .integrate_counts"
	@echo "  help      - Show this help"
	@echo ""
	@echo "Files under test: $(SILICA_FILES)"
