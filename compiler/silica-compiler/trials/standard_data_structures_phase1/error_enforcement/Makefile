LEAF_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
include ../leaf.mk

COMPILE_FAIL_TRIALS := trial_compile_fail_int64_type trial_compile_fail_recursive_type_mismatch trial_compile_fail_unguarded_rec trial_compile_fail_trait_dispatch_no_impl trial_compile_fail_trait_assoc_unresolved trial_compile_fail_trait_missing_required_impl trial_compile_fail_constructor_missing_field trial_compile_fail_constructor_extra_field trial_compile_fail_constructor_witness_mismatch trial_compile_fail_constructor_wrong_module trial_compile_fail_collection_bracket_missing_mem trial_compile_fail_collection_unregistered_module trial_compile_fail_comparator_bare_atom trial_compile_fail_lookup_status_union

.PHONY: compile-fail-integrate record-compile-fail-golden

compile-fail-integrate:
	@cd "$(LEAF_DIR)" || exit 1; \
	failed=0; ok=0; ko=0; \
	if [ ! -x "$(SILICA_COMPILER)" ]; then \
		echo "FAIL: $(MSG_PREFIX)silica-compiler not found at $(SILICA_COMPILER)"; \
		exit 1; \
	fi; \
	for trial in $(COMPILE_FAIL_TRIALS); do \
		if [ ! -f "$(LEAF_DIR)$$trial.golden_fail" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$trial: no .golden_fail (run make record-golden from trial root)"; \
			ko=$$((ko + 1)); failed=1; \
			continue; \
		fi; \
		tmpdir=$$(mktemp -d); \
		ln -s "$(LEAF_DIR)$$trial.silica" "$$tmpdir/$$trial.silica"; \
		if [ -d "$(LEAF_DIR)lib" ]; then \
			mkdir -p "$$tmpdir/lib"; \
			for libf in "$(LEAF_DIR)lib"/*.silica; do \
				[ -f "$$libf" ] || continue; \
				ln -s "$$libf" "$$tmpdir/lib/$$(basename "$$libf")"; \
			done; \
			{ echo "$$trial.silica"; find "$(LEAF_DIR)lib" -name '*.silica' -exec basename {} \; | sort | sed 's|^|lib/|'; } > "$$tmpdir/silica.config"; \
		else \
			echo "$$trial.silica" > "$$tmpdir/silica.config"; \
		fi; \
		(cd "$$tmpdir" && "$(SILICA_COMPILER)") > "$(LEAF_DIR)$$trial.cur_fail" 2>&1 || true; \
		rm -rf "$$tmpdir"; \
		if diff -q "$(LEAF_DIR)$$trial.cur_fail" "$(LEAF_DIR)$$trial.golden_fail" > /dev/null 2>&1; then \
			echo "✅✅ $(MSG_PREFIX)$$trial compile-fail"; \
			ok=$$((ok + 1)); \
		else \
			echo "❌❌ $(MSG_PREFIX)$$trial .cur_fail differs from .golden_fail"; \
			diff "$(LEAF_DIR)$$trial.cur_fail" "$(LEAF_DIR)$$trial.golden_fail" || true; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
		rm -f "$(LEAF_DIR)$$trial.cur_fail"; \
	done; \
	if [ -f "$(LEAF_DIR).integrate_counts" ]; then \
		read p_ok p_ko < "$(LEAF_DIR).integrate_counts" || true; \
		ok=$$((ok + $${p_ok:-0})); ko=$$((ko + $${p_ko:-0})); \
	fi; \
	printf '%d %d\n' "$$ok" "$$ko" > "$(LEAF_DIR).integrate_counts"; \
	exit $$failed

record-compile-fail-golden:
	@if [ ! -x "$(SILICA_COMPILER)" ]; then \
		echo "FAIL: build silica-compiler first: make -C $(abspath $(LEAF_DIR)../../../src)"; \
		exit 1; \
	fi
	@for trial in $(COMPILE_FAIL_TRIALS); do \
		tmpdir=$$(mktemp -d); \
		ln -s "$(LEAF_DIR)$$trial.silica" "$$tmpdir/$$trial.silica"; \
		if [ -d "$(LEAF_DIR)lib" ]; then \
			mkdir -p "$$tmpdir/lib"; \
			for libf in "$(LEAF_DIR)lib"/*.silica; do \
				[ -f "$$libf" ] || continue; \
				ln -s "$$libf" "$$tmpdir/lib/$$(basename "$$libf")"; \
			done; \
			{ echo "$$trial.silica"; find "$(LEAF_DIR)lib" -name '*.silica' -exec basename {} \; | sort | sed 's|^|lib/|'; } > "$$tmpdir/silica.config"; \
		else \
			echo "$$trial.silica" > "$$tmpdir/silica.config"; \
		fi; \
		(cd "$$tmpdir" && "$(SILICA_COMPILER)") > "$(LEAF_DIR)$$trial.golden_fail" 2>&1 || true; \
		rm -rf "$$tmpdir"; \
		echo "Recorded $$trial.golden_fail"; \
	done

integrate: positive-integrate compile-fail-integrate

record-golden: record-positive-golden record-compile-fail-golden
