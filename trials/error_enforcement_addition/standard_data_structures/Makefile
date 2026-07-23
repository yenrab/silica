# Compile-fail trials for Phase 1 standard data structures (constructor/trait/type checks).
# Each trial is compiled in an isolated temp directory with lib/ stubs symlinked in;
# compiler stderr/stdout is diffed against the matching .golden_fail.

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
include $(THIS_DIR)../../silica_compiler.mk

TRIAL_DIR := error_enforcement_addition/standard_data_structures
MSG_PREFIX := $(TRIAL_DIR)/

COMPILE_FAIL_TRIALS := \
	trial_compile_fail_int64_type \
	trial_compile_fail_recursive_type_mismatch \
	trial_compile_fail_unguarded_rec \
	trial_compile_fail_trait_dispatch_no_impl \
	trial_compile_fail_trait_assoc_unresolved \
	trial_compile_fail_trait_missing_required_impl \
	trial_compile_fail_constructor_missing_field \
	trial_compile_fail_constructor_extra_field \
	trial_compile_fail_constructor_witness_mismatch \
	trial_compile_fail_constructor_wrong_module \
	trial_compile_fail_collection_bracket_missing_mem \
	trial_compile_fail_collection_unregistered_module \
	trial_compile_fail_comparator_bare_atom \
	trial_compile_fail_lookup_status_union \
	trial_compile_fail_ordered_map_constructor_missing_field \
	trial_compile_fail_ordered_map_constructor_extra_field \
	trial_compile_fail_ordered_set_constructor_missing_field \
	trial_compile_fail_ordered_set_constructor_extra_field

.DEFAULT_GOAL := all
.PHONY: all integrate clean record-golden help

all: integrate

integrate:
	@cd "$(THIS_DIR)" || exit 1; \
	failed=0; ok=0; ko=0; rm -f "$(THIS_DIR).integrate_counts"; \
	$(ENSURE_SILICA_COMPILER); \
	for trial in $(COMPILE_FAIL_TRIALS); do \
		if [ ! -f "$(THIS_DIR)$$trial.golden_fail" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$trial: no .golden_fail (run make record-golden)"; \
			ko=$$((ko + 1)); failed=1; \
			continue; \
		fi; \
		tmpdir=$$(mktemp -d); \
		ln -s "$(THIS_DIR)$$trial.silica" "$$tmpdir/$$trial.silica"; \
		if [ -d "$(THIS_DIR)lib" ]; then \
			mkdir -p "$$tmpdir/lib"; \
			for libf in "$(THIS_DIR)lib"/*.silica; do \
				[ -f "$$libf" ] || continue; \
				ln -s "$$libf" "$$tmpdir/lib/$$(basename "$$libf")"; \
			done; \
			{ echo "$$trial.silica"; find "$(THIS_DIR)lib" \( -type f -o -type l \) -name '*.silica' -exec basename {} \; | sort | sed 's|^|lib/|'; } > "$$tmpdir/silica.config"; \
		else \
			echo "$$trial.silica" > "$$tmpdir/silica.config"; \
		fi; \
		(cd "$$tmpdir" && { $(call RUN_SILICA_COMPILER_QUIET_WITH,"$(SILICA_COMPILER)"); }) > "$(THIS_DIR)$$trial.cur_fail" 2>&1 || true; \
		rm -rf "$$tmpdir"; \
		if diff -q "$(THIS_DIR)$$trial.cur_fail" "$(THIS_DIR)$$trial.golden_fail" > /dev/null 2>&1; then \
			echo "✅✅ $(MSG_PREFIX)$$trial"; \
			ok=$$((ok + 1)); \
		else \
			echo "❌❌ $(MSG_PREFIX)$$trial: .cur_fail differs from .golden_fail"; \
			diff "$(THIS_DIR)$$trial.cur_fail" "$(THIS_DIR)$$trial.golden_fail" || true; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
		rm -f "$(THIS_DIR)$$trial.cur_fail"; \
	done; \
	printf '%d %d\n' "$$ok" "$$ko" > "$(THIS_DIR).integrate_counts"; \
	exit $$failed

record-golden:
	@$(ENSURE_SILICA_COMPILER)
	@for trial in $(COMPILE_FAIL_TRIALS); do \
		tmpdir=$$(mktemp -d); \
		ln -s "$(THIS_DIR)$$trial.silica" "$$tmpdir/$$trial.silica"; \
		if [ -d "$(THIS_DIR)lib" ]; then \
			mkdir -p "$$tmpdir/lib"; \
			for libf in "$(THIS_DIR)lib"/*.silica; do \
				[ -f "$$libf" ] || continue; \
				ln -s "$$libf" "$$tmpdir/lib/$$(basename "$$libf")"; \
			done; \
			{ echo "$$trial.silica"; find "$(THIS_DIR)lib" \( -type f -o -type l \) -name '*.silica' -exec basename {} \; | sort | sed 's|^|lib/|'; } > "$$tmpdir/silica.config"; \
		else \
			echo "$$trial.silica" > "$$tmpdir/silica.config"; \
		fi; \
		(cd "$$tmpdir" && { $(call RUN_SILICA_COMPILER_QUIET_WITH,"$(SILICA_COMPILER)"); }) > "$(THIS_DIR)$$trial.golden_fail" 2>&1 || true; \
		rm -rf "$$tmpdir"; \
		echo "Recorded $$trial.golden_fail"; \
	done

clean:
	@rm -f "$(THIS_DIR)"*.cur_fail "$(THIS_DIR).integrate_counts"
	@echo "✅ $(MSG_PREFIX)clean complete"

help:
	@echo "error_enforcement_addition/standard_data_structures"
	@echo "===================================================="
	@echo "Compile-fail goldens for Phase 1 standard data structures."
	@echo ""
	@echo "Targets:"
	@echo "  integrate     - Diff compiler output against .golden_fail (default)"
	@echo "  record-golden - Capture current compiler output as .golden_fail"
	@echo "  clean         - Remove .cur_fail and .integrate_counts"
	@echo "  help          - Show this help"
