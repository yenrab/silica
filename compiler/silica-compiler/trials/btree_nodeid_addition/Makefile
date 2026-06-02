# Phase 7 — NodeIDBTreeMap trials.
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
	btree_nodeid_empty_get.silica \
	btree_nodeid_insert_one.silica \
	btree_nodeid_insert_two.silica \
	btree_nodeid_insert_four.silica \
	btree_nodeid_insert_get.silica \
	btree_nodeid_insert.silica \
	btree_nodeid_validate_invalid.silica \
	btree_nodeid_context_trial.silica

LIB_SRC := ../../src/standard_data_structures

.DEFAULT_GOAL := all
.PHONY: all clean assembly objects executables integrate help silica.config lib-sources

lib-sources:
	@mkdir -p "$(THIS_DIR)lib"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_nodeid.silica)" "$(THIS_DIR)lib/btree_nodeid.silica"

silica.config: lib-sources
	@cd "$(THIS_DIR)" && { \
		echo 'lib/btree_nodeid.silica'; \
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
		lib_objs="lib/btree_nodeid.o"; \
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
	rm -f *.sams lib/*.sams; \
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
		lib_objs="lib/btree_nodeid.o"; \
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
		btree_nodeid_empty_get \
		btree_nodeid_insert_one \
		btree_nodeid_insert_two \
		btree_nodeid_insert_four \
		btree_nodeid_insert_get \
		btree_nodeid_insert \
		btree_nodeid_validate_invalid \
		silica.config .integrate_counts; \
	rm -f lib/*.sams lib/*.o
	@echo "✅ $(MSG_PREFIX)Clean complete"

help:
	@echo "btree_nodeid_addition — NodeIDBTreeMap trials (Phase 7)"
