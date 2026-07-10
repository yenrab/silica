LEAF_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
THIS_DIR := $(LEAF_DIR)
LIB_SRC := ../../../stdlib/data_structures

STDLIB_PREREQ_MK := $(firstword $(wildcard $(LEAF_DIR)../stdlib_prereq.mk $(LEAF_DIR)../../stdlib_prereq.mk $(LEAF_DIR)../../../stdlib_prereq.mk))
include $(STDLIB_PREREQ_MK)

include $(LEAF_DIR)../leaf.mk

# WBT core intentionally enumerates accepted gate trials.  This leaf is used
# for narrow compiler/runtime probes while developing WBT; loose probe files
# must not become part of the stable integrate gate merely because they have a
# `.silica` suffix.
POSITIVE_SILICA := \
	smoke_harness_ready.silica \
	wbt_balance_boundaries.silica \
	wbt_constructor_ordering_bundle.silica \
	wbt_empty_representation.silica \
	wbt_fold_ascending.silica \
	wbt_generic_payload_shapes.silica \
	wbt_generic_tuple_map_empty.silica \
	wbt_minimum_maximum.silica \
	wbt_read_only_alloc_free.silica \
	wbt_read_only_empty.silica \
	wbt_rebalance_adversarial.silica \
	wbt_representation_specializations.silica \
	wbt_representation_string_specialization.silica \
	wbt_rotation_double_left.silica \
	wbt_rotation_double_right.silica \
	wbt_rotation_gamma_equality.silica \
	wbt_rotation_single_left.silica \
	wbt_rotation_single_right.silica \
	wbt_search_contains.silica \
	wbt_set_insert_adversarial.silica \
	wbt_set_insert_duplicate.silica \
	wbt_set_insert_orders.silica \
	wbt_set_insert_payload_shapes.silica \
	wbt_set_insert_persistence.silica \
	wbt_set_insert_sharing.silica \
	wbt_map_canonical_key.silica \
	wbt_map_compare_value_not_called.silica \
	wbt_map_insert_payload_shapes.silica \
	wbt_map_insert_orders.silica \
	wbt_map_insert_replace.silica \
	wbt_map_insert_tuple_value.silica \
	wbt_map_replace_persistence.silica \
	wbt_map_value_pairing.silica \
	wbt_smart_node_arena.silica \
	wbt_smart_node_production_path.silica \
	wbt_smart_node_size.silica \
	wbt_weight_checked.silica

COLLECTION_ERROR_TRIALS := trial_collection_error_wbt_search_invalid_comparator.silica trial_collection_error_wbt_smart_node_overflow.silica trial_collection_error_wbt_balance_missing_child.silica trial_collection_error_wbt_set_insert_invalid_comparator.silica trial_collection_error_wbt_map_insert_invalid_comparator.silica
ALL_COMPILE_SILICA := $(POSITIVE_SILICA) $(COLLECTION_ERROR_TRIALS)

# §8A.3 exit gate: production WBT modules must contain exactly one node allocation site (inside smart_node).
WBT_PRODUCTION_MODULES := wbt_set wbt_map

.PHONY: lib-sources check-wbt-alloc-rec-gate

check-wbt-alloc-rec-gate:
	@failed=0; \
	for mod in $(WBT_PRODUCTION_MODULES); do \
		silica="$(LEAF_DIR)lib/$$mod.silica"; \
		sams="$(LEAF_DIR)lib/$$mod.sams"; \
		if [ ! -f "$$silica" ]; then \
			echo "FAIL: $(MSG_PREFIX)missing $$silica (run make lib-sources)"; \
			failed=1; \
			continue; \
		fi; \
		if [ ! -f "$$sams" ]; then \
			echo "FAIL: $(MSG_PREFIX)missing $$sams (compile stdlib modules first)"; \
			failed=1; \
			continue; \
		fi; \
		src_count=$$(grep -c 'alloc_rec' "$$silica" || true); \
		if [ "$$src_count" -ne 1 ]; then \
			echo "FAIL: $(MSG_PREFIX)$$mod.silica has $$src_count alloc_rec sites (expected 1 in smart_node_finish)"; \
			grep -n 'alloc_rec' "$$silica" || true; \
			failed=1; \
		else \
			echo "✅ $(MSG_PREFIX)$$mod.silica single smart_node allocation site"; \
		fi; \
	done; \
	exit $$failed

lib-sources: stdlib-data-structures
	@mkdir -p "$(LEAF_DIR)lib"
	@ln -sf "$(abspath $(LEAF_DIR)$(LIB_SRC)/wbt_set.silica)" "$(LEAF_DIR)lib/wbt_set.silica"
	@ln -sf "$(abspath $(LEAF_DIR)$(LIB_SRC)/wbt_map.silica)" "$(LEAF_DIR)lib/wbt_map.silica"

silica.config: lib-sources
	@cd "$(LEAF_DIR)" && { \
		printf '%s\n' $(ALL_COMPILE_SILICA); \
		if [ -d lib ]; then find lib -name '*.silica' | sort; fi; \
	} > silica.config

assembly: lib-sources silica.config
	@if [ ! -s "$(LEAF_DIR)silica.config" ]; then \
		echo "SKIP: $(MSG_PREFIX)no positive .silica trials"; \
		exit 0; \
	fi
	@if [ ! -x "$(SILICA_COMPILER)" ]; then \
		echo "FAIL: $(MSG_PREFIX)silica-compiler not found at $(SILICA_COMPILER)"; \
		exit 1; \
	fi
	@$(INTEGRATE_PRE_CLEAN)
	@echo "Compiling with silica-compiler..."
	@cd "$(LEAF_DIR)" && "$(SILICA_COMPILER)"
	@echo "✅ $(MSG_PREFIX)Assembly generated"

positive-integrate: lib-sources silica.config
	@echo "$(MSG_PREFIX)positive-integrate: starting..."
	@cd "$(LEAF_DIR)" || exit 1; \
	$(INTEGRATE_PRE_CLEAN); \
	ok=0; ko=0; failed=0; \
	if [ ! -s silica.config ]; then \
		echo "SKIP: $(MSG_PREFIX)no positive trials"; \
		printf '%d %d\n' 0 0 > .integrate_counts; \
		exit 0; \
	fi; \
	if [ ! -x "$(SILICA_COMPILER)" ]; then \
		echo "FAIL: $(MSG_PREFIX)silica-compiler not found at $(SILICA_COMPILER)"; \
		printf '%d %d\n' 0 1 > .integrate_counts; \
		exit 1; \
	fi; \
	echo "Compiling with silica-compiler..."; \
	if ! "$(SILICA_COMPILER)"; then \
		echo "FAIL: $(MSG_PREFIX)compilation failed"; \
		printf '%d %d\n' 0 1 > .integrate_counts; \
		exit 1; \
	fi; \
	$(MAKE) check-wbt-alloc-rec-gate || { failed=1; printf '%d %d\n' 0 1 > .integrate_counts; exit 1; }; \
	for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		if [ ! -f "$$base.ascomp" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$base has no .ascomp file (run make record-golden from trial root)"; \
			ko=$$((ko + 1)); failed=1; \
		elif ! diff -Bw -q "$$sams" "$$base.ascomp" > /dev/null 2>&1; then \
			echo "❌❌ $(MSG_PREFIX)$$base .sams differs from .ascomp"; \
			diff -Bw "$$sams" "$$base.ascomp" || true; \
			ko=$$((ko + 1)); failed=1; \
		else \
			echo "✅✅ $(MSG_PREFIX)$$base assembly matches .ascomp"; \
			ok=$$((ok + 1)); \
		fi; \
	done; \
	for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		echo "Assembling $$sams..."; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o" || { failed=1; break; }; \
	done; \
	if [ -f "__silica_runtime.sams" ]; then \
		echo "Assembling __silica_runtime.sams..."; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "__silica_runtime.sams" -o "__silica_runtime.o" || failed=1; \
	fi; \
	for sams in lib/*.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$$(basename "$$sams" .sams); \
		echo "Assembling $$sams..."; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "lib/$$base.o" || failed=1; \
	done; \
	lib_objs=""; \
	for obj in lib/*.o; do [ -f "$$obj" ] && lib_objs="$$lib_objs $$obj"; done; \
	rust_sysroot=$$(rustc --print sysroot 2>/dev/null); \
	rust_target=$$(rustc -vV 2>/dev/null | sed -n 's/^host: //p'); \
	rust_lld="$$rust_sysroot/lib/rustlib/$$rust_target/bin/rust-lld"; \
	macos_sdk=$$(xcrun --sdk macosx --show-sdk-path 2>/dev/null); \
	for trial in $(ALL_COMPILE_SILICA); do \
		base=$${trial%.silica}; \
		[ -f "$$base.o" ] || continue; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
		echo "Linking $$base..."; \
		if test -x "$$rust_lld"; then \
			$$rust_lld -flavor darwin -o "$$base" "$$base.o" $$lib_objs $$runtime_obj \
				-arch arm64 -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) \
				-syslibroot "$$macos_sdk" -lSystem -e main || failed=1; \
		else \
			$(ASSEMBLER) "$$base.o" $$lib_objs $$runtime_obj -o "$$base" $(LDFLAGS_clang) || failed=1; \
		fi; \
	done; \
	for trial in $(ALL_COMPILE_SILICA); do \
		base=$${trial%.silica}; \
		[ -x "$$base" ] || continue; \
		echo "Running $$base..."; \
		{ ./$$base 2>&1; echo $$?; } > "$$base.sout"; \
		if [ -f "$$base.scout" ]; then \
			if ! diff -Bw -q "$$base.sout" "$$base.scout" > /dev/null 2>&1; then \
				echo "❌❌ $(MSG_PREFIX)$$base .sout differs from .scout"; \
				diff -Bw "$$base.sout" "$$base.scout" || true; \
				ko=$$((ko + 1)); failed=1; \
			else \
				echo "✅✅ $(MSG_PREFIX)$$base output matches .scout"; \
				ok=$$((ok + 1)); \
			fi; \
		else \
			echo "❌❌ $(MSG_PREFIX)$$base has no .scout file (run make record-golden from trial root)"; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
	done; \
	[ "$$failed" -ne 0 ] && [ "$$ko" -eq 0 ] && ko=$$((ko + 1)); \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	exit $$failed

record-positive-golden: lib-sources silica.config
	@echo "$(MSG_PREFIX)record-positive-golden: starting..."
	@if [ ! -x "$(SILICA_COMPILER)" ]; then \
		echo "FAIL: build silica-compiler first: make -C $(LEAF_DIR)../../../src"; \
		exit 1; \
	fi
	@$(INTEGRATE_PRE_CLEAN)
	@cd "$(LEAF_DIR)" && "$(SILICA_COMPILER)"
	@cd "$(LEAF_DIR)" && for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		cp "$$sams" "$$base.ascomp"; \
		echo "Recorded $$base.ascomp"; \
	done
	@cd "$(LEAF_DIR)" && for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o"; \
	done
	@if [ -f "$(LEAF_DIR)__silica_runtime.sams" ]; then \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$(LEAF_DIR)__silica_runtime.sams" -o "$(LEAF_DIR)__silica_runtime.o"; \
	fi
	@cd "$(LEAF_DIR)" && for sams in lib/*.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$$(basename "$$sams" .sams); \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "lib/$$base.o"; \
	done
	@cd "$(LEAF_DIR)" || exit 1; \
	lib_objs=""; \
	for obj in lib/*.o; do [ -f "$$obj" ] && lib_objs="$$lib_objs $$obj"; done; \
	rust_sysroot=$$(rustc --print sysroot 2>/dev/null); \
	rust_target=$$(rustc -vV 2>/dev/null | sed -n 's/^host: //p'); \
	rust_lld="$$rust_sysroot/lib/rustlib/$$rust_target/bin/rust-lld"; \
	macos_sdk=$$(xcrun --sdk macosx --show-sdk-path 2>/dev/null); \
	for trial in $(ALL_COMPILE_SILICA); do \
		base=$${trial%.silica}; \
		[ -f "$$base.o" ] || continue; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
		if test -x "$$rust_lld"; then \
			$$rust_lld -flavor darwin -o "$$base" "$$base.o" $$lib_objs $$runtime_obj \
				-arch arm64 -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) \
				-syslibroot "$$macos_sdk" -lSystem -e main; \
		else \
			$(ASSEMBLER) "$$base.o" $$lib_objs $$runtime_obj -o "$$base" $(LDFLAGS_clang); \
		fi; \
		{ ./$$base 2>&1; echo $$?; } > "$$base.scout"; \
		echo "Recorded $$base.scout"; \
	done
	@cd "$(LEAF_DIR)" && for trial in $(ALL_COMPILE_SILICA); do \
		base=$${trial%.silica}; \
		rm -f "$$base" "$$base.o"; \
	done
	@cd "$(LEAF_DIR)" && rm -f __silica_runtime.o lib/*.o *.sams lib/*.sams *.sout .integrate_counts
	@echo "✅ $(MSG_PREFIX)record-positive-golden complete"

clean:
	@echo "$(MSG_PREFIX)clean: removing prior artifacts..."
	@cd "$(LEAF_DIR)" && for s in *.sams; do \
		[ -f "$$s" ] || continue; \
		rm -f "$${s%.sams}"; \
	done
	@cd "$(LEAF_DIR)" && rm -f *.sams lib/wbt_set.sams lib/wbt_map.sams lib/wbt_set.silica lib/wbt_map.silica *.o lib/*.o __silica_runtime.o smoke_harness_ready wbt_balance_boundaries wbt_empty_representation wbt_generic_payload_shapes wbt_generic_tuple_map_empty wbt_representation_specializations wbt_representation_string_specialization wbt_constructor_ordering_bundle wbt_search_contains wbt_minimum_maximum wbt_fold_ascending wbt_read_only_alloc_free wbt_read_only_empty wbt_rebalance_adversarial wbt_rotation_double_left wbt_rotation_double_right wbt_rotation_gamma_equality wbt_rotation_single_left wbt_rotation_single_right wbt_smart_node_arena wbt_smart_node_production_path wbt_smart_node_size wbt_weight_checked wbt_set_insert_adversarial wbt_set_insert_duplicate wbt_set_insert_orders wbt_set_insert_payload_shapes wbt_set_insert_persistence wbt_set_insert_sharing trial_collection_error_wbt_search_invalid_comparator trial_collection_error_wbt_smart_node_overflow trial_collection_error_wbt_balance_missing_child trial_collection_error_wbt_set_insert_invalid_comparator *.sout silica.config .integrate_counts
	@echo "✅ $(MSG_PREFIX)Clean complete"
