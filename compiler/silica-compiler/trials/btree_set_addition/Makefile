# Phase 5–8 — NodeIDBTreeSet, CsrBTreeSet, and CsrBTreeMap trials.
THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER := $(THIS_DIR)../../src/silica-compiler
TRIAL_DIR := $(notdir $(patsubst %/,%,$(THIS_DIR)))
MSG_PREFIX := $(TRIAL_DIR)/

RUST_SYSROOT := $(shell rustc --print sysroot 2>/dev/null)
RUST_TARGET := $(shell rustc -vV 2>/dev/null | sed -n "s/^host: *//p")
RUST_LLD := $(RUST_SYSROOT)/lib/rustlib/$(RUST_TARGET)/bin/rust-lld
LINKER := $(shell test -x "$(RUST_LLD)" && echo "$(RUST_LLD)" || echo "clang")
ASSEMBLER := clang

MACOS_SDK := $(shell xcrun --sdk macosx --show-sdk-path 2>/dev/null)
MACOS_MIN_VERSION ?= 26.0
ASFLAGS_macos := -mmacosx-version-min=$(MACOS_MIN_VERSION)
LDFLAGS_rust-lld := -arch arm64 -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) -syslibroot $(MACOS_SDK) -lSystem -e main
LDFLAGS_clang := -Wl,-e,main -Wl,-macos_version_min,$(MACOS_MIN_VERSION)

TRIAL_SOURCES := \
	btree_set_nodeid_empty_contains.silica \
	btree_set_nodeid_contains_handbuilt.silica \
	btree_set_nodeid_insert.silica \
	btree_set_nodeid_insert_split.silica \
	btree_set_nodeid_validate_invalid.silica \
	btree_set_csr_contains_static.silica \
	btree_set_csr_validate_invalid.silica \
	btree_set_csr_insert.silica \
	btree_set_nodeid_to_csr.silica \
	btree_csr_map_contains_static.silica \
	btree_csr_map_validate_invalid.silica \
	btree_csr_map_insert.silica \
	btree_csr_map_insert_split.silica \
	btree_nodeid_map_to_csr.silica

TRIAL_BINARIES := $(TRIAL_SOURCES:.silica=)

LIB_SRC := ../../stdlib/data_structures

STDLIB_PREREQ_MK := $(firstword $(wildcard $(THIS_DIR)../stdlib_prereq.mk $(THIS_DIR)../../stdlib_prereq.mk $(THIS_DIR)../../../stdlib_prereq.mk))
include $(STDLIB_PREREQ_MK)

.DEFAULT_GOAL := all
.PHONY: all clean assembly objects executables integrate help silica.config lib-sources stdlib-data-structures

lib-sources: stdlib-data-structures
	@mkdir -p "$(THIS_DIR)lib"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_set_csr.silica)" "$(THIS_DIR)lib/btree_set_csr.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_set_nodeid.silica)" "$(THIS_DIR)lib/btree_set_nodeid.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_nodeid.silica)" "$(THIS_DIR)lib/btree_nodeid.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_csr_map.silica)" "$(THIS_DIR)lib/btree_csr_map.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/OrderedSet.silica)" "$(THIS_DIR)lib/OrderedSet.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/OrderedMap.silica)" "$(THIS_DIR)lib/OrderedMap.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/ordered_set_nodeid_adapter.silica)" "$(THIS_DIR)lib/ordered_set_nodeid_adapter.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/ordered_set_csr_adapter.silica)" "$(THIS_DIR)lib/ordered_set_csr_adapter.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/ordered_map_nodeid_adapter.silica)" "$(THIS_DIR)lib/ordered_map_nodeid_adapter.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/ordered_map_csr_adapter.silica)" "$(THIS_DIR)lib/ordered_map_csr_adapter.silica"

silica.config: lib-sources
	@cd "$(THIS_DIR)" && { \
		echo 'lib/btree_set_csr.silica'; \
		echo 'lib/btree_set_nodeid.silica'; \
		echo 'lib/btree_nodeid.silica'; \
		echo 'lib/btree_csr_map.silica'; \
		echo 'lib/OrderedSet.silica'; \
		echo 'lib/OrderedMap.silica'; \
		echo 'lib/ordered_set_nodeid_adapter.silica'; \
		echo 'lib/ordered_set_csr_adapter.silica'; \
		echo 'lib/ordered_map_nodeid_adapter.silica'; \
		echo 'lib/ordered_map_csr_adapter.silica'; \
		echo 'lib/csr_set_trait_bridge.silica'; \
		echo 'lib/csr_map_trait_bridge.silica'; \
		for src in $(TRIAL_SOURCES); do echo "$$src"; done; \
	} > silica.config

all: executables

assembly: silica.config
	@echo "Compiling with silica-compiler..."
	@cd "$(THIS_DIR)" && $(SILICA_COMPILER)
	@echo "✅ $(MSG_PREFIX)Assembly generated"

objects: assembly
	@cd "$(THIS_DIR)" && \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$'); do \
		base=$${line%.silica}; \
		sams="$$base.sams"; \
		[ -f "$$sams" ] || continue; \
		echo "Assembling: $$sams -> $$base.o"; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o" || exit 1; \
	done; \
	if [ -f "__silica_runtime.sams" ]; then \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "__silica_runtime.sams" -o "__silica_runtime.o"; \
	fi

executables: objects
	@echo "Linking..."
	@cd "$(THIS_DIR)" && \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^lib/'); do \
		base=$${line%.silica}; \
		[ -f "$$base.o" ] || continue; \
		$(COLLECT_LIB_OBJS); \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o "$$base" "$$base.o" $$lib_objs $$runtime_obj $(LDFLAGS_rust-lld) && echo "  ✅ $(MSG_PREFIX)$$base"; \
		else \
			$(ASSEMBLER) "$$base.o" $$lib_objs $$runtime_obj -o "$$base" $(LDFLAGS_clang) && echo "  ✅ $(MSG_PREFIX)$$base"; \
		fi; \
	done

integrate: silica.config
	@rm -f .integrate_counts; ok=0; ko=0; failed=0; \
	rm -f *.sams lib/*.sams $(TRIAL_BINARIES); \
	echo "Compiling with silica-compiler..."; \
	if ! $(abspath $(SILICA_COMPILER)); then \
		echo "FAIL: Compilation failed"; \
		printf '%d %d\n' 0 1 > .integrate_counts; \
		exit 1; \
	fi; \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$'); do \
		base=$${line%.silica}; \
		sams="$$base.sams"; \
		[ -f "$$sams" ] || continue; \
		echo "Assembling $$sams..."; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o" || { failed=1; break; }; \
	done; \
	if [ -f "__silica_runtime.sams" ]; then \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "__silica_runtime.sams" -o "__silica_runtime.o" || failed=1; \
	fi; \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^lib/'); do \
		base=$${line%.silica}; \
		[ -f "$$base.o" ] || continue; \
		$(COLLECT_LIB_OBJS); \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
		echo "Linking $$base..."; \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o "$$base" "$$base.o" $$lib_objs $$runtime_obj $(LDFLAGS_rust-lld) || failed=1; \
		else \
			$(ASSEMBLER) "$$base.o" $$lib_objs $$runtime_obj -o "$$base" $(LDFLAGS_clang) || failed=1; \
		fi; \
	done; \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^lib/'); do \
		base=$${line%.silica}; \
		if [ -x "$$base" ]; then \
			{ ./"$$base" 2>&1; echo $$?; } > "$$base.sout" & \
			run_pid=$$!; elapsed=0; \
			while kill -0 "$$run_pid" 2>/dev/null && [ "$$elapsed" -lt 5 ]; do \
				sleep 1; elapsed=$$((elapsed + 1)); \
			done; \
			if kill -0 "$$run_pid" 2>/dev/null; then \
				kill -9 "$$run_pid" 2>/dev/null || true; \
				wait "$$run_pid" 2>/dev/null || true; \
				echo "TIMEOUT" > "$$base.sout"; \
			else \
				wait "$$run_pid" 2>/dev/null || true; \
			fi; \
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
				echo "❌❌ $(MSG_PREFIX)$$base has no .scout file"; \
				ko=$$((ko + 1)); failed=1; \
			fi; \
		fi; \
	done; \
	[ "$$failed" -ne 0 ] && [ "$$ko" -eq 0 ] && ko=$$((ko + 1)); \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	exit $$failed

clean:
	@cd "$(THIS_DIR)" && \
	rm -f *.sout *.sams *.o __silica_runtime.o __silica_runtime.sams \
		$(TRIAL_BINARIES) \
		silica.config .integrate_counts; \
	rm -f lib/*.sams lib/*.o
	@echo "✅ $(MSG_PREFIX)Clean complete"

help:
	@echo "btree_set_addition — B-tree set and map trials (Phases 5–8)"
