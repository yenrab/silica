LEAF_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
THIS_DIR := $(LEAF_DIR)
LIB_SRC := ../../../stdlib/data_structures

STDLIB_PREREQ_MK := $(firstword $(wildcard $(LEAF_DIR)../stdlib_prereq.mk $(LEAF_DIR)../../stdlib_prereq.mk $(LEAF_DIR)../../../stdlib_prereq.mk))
include $(STDLIB_PREREQ_MK)

include $(LEAF_DIR)../leaf.mk

COLLECTION_ERROR_TRIALS := trial_collection_error_wbt_search_invalid_comparator.silica
ALL_COMPILE_SILICA := $(POSITIVE_SILICA) $(COLLECTION_ERROR_TRIALS)

.PHONY: lib-sources

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
	@cd "$(LEAF_DIR)" && rm -f *.sams lib/wbt_set.sams lib/wbt_map.sams lib/wbt_set.silica lib/wbt_map.silica *.o lib/*.o __silica_runtime.o smoke_harness_ready wbt_empty_representation wbt_generic_payload_shapes wbt_representation_specializations wbt_constructor_ordering_bundle wbt_search_contains wbt_minimum_maximum wbt_fold_ascending wbt_read_only_alloc_free wbt_read_only_empty trial_collection_error_wbt_search_invalid_comparator *.sout silica.config .integrate_counts
	@echo "✅ $(MSG_PREFIX)Clean complete"
