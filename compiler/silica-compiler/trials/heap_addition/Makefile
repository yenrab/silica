# Phase 9 — RegionBinaryHeap generated modules (balanced_tree_and_heap_design.md §6).
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

MIN_TRIAL_SOURCES := \
	heap_binary_min_empty.silica \
	heap_binary_min_push_pop.silica \
	heap_binary_min_validate_invalid.silica \
	heap_binary_min_priority_push_pop.silica

MAX_TRIAL_SOURCES := \
	heap_binary_max_push_pop.silica

LIB_SRC := ../../src/standard_data_structures

.DEFAULT_GOAL := all
.PHONY: all clean assembly objects executables integrate help lib-sources silica.config.min silica.config.max

lib-sources:
	@mkdir -p "$(THIS_DIR)lib"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/heap_binary_min.silica)" "$(THIS_DIR)lib/heap_binary_min.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/heap_binary_max.silica)" "$(THIS_DIR)lib/heap_binary_max.silica"

silica.config.min: lib-sources
	@cd "$(THIS_DIR)" && { \
		echo 'lib/heap_binary_min.silica'; \
		for src in $(MIN_TRIAL_SOURCES); do echo "$$src"; done; \
	} > silica.config.min

silica.config.max: lib-sources
	@cd "$(THIS_DIR)" && { \
		echo 'lib/heap_binary_max.silica'; \
		for src in $(MAX_TRIAL_SOURCES); do echo "$$src"; done; \
	} > silica.config.max

silica.config: silica.config.min silica.config.max
	@cd "$(THIS_DIR)" && cat silica.config.min silica.config.max > silica.config

all: executables

assembly: silica.config.min silica.config.max
	@echo "Compiling min-heap trials with silica-compiler..."
	@cd "$(THIS_DIR)" && cp silica.config.min silica.config && $(SILICA_COMPILER)
	@echo "Compiling max-heap trials with silica-compiler..."
	@cd "$(THIS_DIR)" && cp silica.config.max silica.config && $(SILICA_COMPILER)
	@echo "✅ $(MSG_PREFIX)Assembly generated"

objects: assembly
	@cd "$(THIS_DIR)" && \
	for line in $$(grep -v '^#' silica.config.min 2>/dev/null | grep -v '^$$'); do \
		base=$${line%.silica}; \
		sams="$$base.sams"; \
		[ -f "$$sams" ] || continue; \
		echo "Assembling: $$sams -> $$base.o"; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o" || exit 1; \
	done; \
	for line in $$(grep -v '^#' silica.config.max 2>/dev/null | grep -v '^$$'); do \
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
	for line in $$(grep -v '^#' silica.config.min 2>/dev/null | grep -v '^$$' | grep -v '^lib/'); do \
		base=$${line%.silica}; \
		[ -f "$$base.o" ] || continue; \
		lib_objs="lib/heap_binary_min.o"; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o "$$base" "$$base.o" $$lib_objs $$runtime_obj $(LDFLAGS_rust-lld) && echo "  ✅ $(MSG_PREFIX)$$base"; \
		else \
			$(ASSEMBLER) "$$base.o" $$lib_objs $$runtime_obj -o "$$base" $(LDFLAGS_clang) && echo "  ✅ $(MSG_PREFIX)$$base"; \
		fi; \
	done; \
	for line in $$(grep -v '^#' silica.config.max 2>/dev/null | grep -v '^$$' | grep -v '^lib/'); do \
		base=$${line%.silica}; \
		[ -f "$$base.o" ] || continue; \
		lib_objs="lib/heap_binary_max.o"; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o "$$base" "$$base.o" $$lib_objs $$runtime_obj $(LDFLAGS_rust-lld) && echo "  ✅ $(MSG_PREFIX)$$base"; \
		else \
			$(ASSEMBLER) "$$base.o" $$lib_objs $$runtime_obj -o "$$base" $(LDFLAGS_clang) && echo "  ✅ $(MSG_PREFIX)$$base"; \
		fi; \
	done

integrate: lib-sources silica.config.min silica.config.max
	@rm -f .integrate_counts; ok=0; ko=0; failed=0; \
	rm -f *.sams lib/*.sams; \
	echo "Compiling min-heap trials..."; \
	if ! (cd "$(THIS_DIR)" && cp silica.config.min silica.config && $(abspath $(SILICA_COMPILER))); then \
		echo "FAIL: min-heap compilation failed"; \
		printf '%d %d\n' 0 1 > .integrate_counts; \
		exit 1; \
	fi; \
	echo "Compiling max-heap trials..."; \
	if ! (cd "$(THIS_DIR)" && cp silica.config.max silica.config && $(abspath $(SILICA_COMPILER))); then \
		echo "FAIL: max-heap compilation failed"; \
		printf '%d %d\n' 0 1 > .integrate_counts; \
		exit 1; \
	fi; \
	for cfg in silica.config.min silica.config.max; do \
		for line in $$(grep -v '^#' "$(THIS_DIR)/$$cfg" 2>/dev/null | grep -v '^$$'); do \
			base=$${line%.silica}; \
			sams="$(THIS_DIR)/$$base.sams"; \
			[ -f "$$sams" ] || continue; \
			echo "Assembling $$sams..."; \
			$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$(THIS_DIR)/$$base.o" || { failed=1; break; }; \
		done; \
	done; \
	if [ -f "$(THIS_DIR)/__silica_runtime.sams" ]; then \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$(THIS_DIR)/__silica_runtime.sams" -o "$(THIS_DIR)/__silica_runtime.o" || failed=1; \
	fi; \
	for cfg in silica.config.min silica.config.max; do \
		lib_obj="lib/heap_binary_min.o"; \
		case "$$cfg" in *max*) lib_obj="lib/heap_binary_max.o";; esac; \
		for line in $$(grep -v '^#' "$(THIS_DIR)/$$cfg" 2>/dev/null | grep -v '^$$' | grep -v '^lib/'); do \
			base=$${line%.silica}; \
			[ -f "$(THIS_DIR)/$$base.o" ] || continue; \
			runtime_obj=""; \
			[ -f "$(THIS_DIR)/__silica_runtime.o" ] && runtime_obj="$(THIS_DIR)/__silica_runtime.o"; \
			echo "Linking $$base..."; \
			if test -x "$(RUST_LLD)"; then \
				$(RUST_LLD) -flavor darwin -o "$(THIS_DIR)/$$base" "$(THIS_DIR)/$$base.o" "$(THIS_DIR)/$$lib_obj" $$runtime_obj $(LDFLAGS_rust-lld) || failed=1; \
			else \
				$(ASSEMBLER) "$(THIS_DIR)/$$base.o" "$(THIS_DIR)/$$lib_obj" $$runtime_obj -o "$(THIS_DIR)/$$base" $(LDFLAGS_clang) || failed=1; \
			fi; \
		done; \
	done; \
	for cfg in silica.config.min silica.config.max; do \
		for line in $$(grep -v '^#' "$(THIS_DIR)/$$cfg" 2>/dev/null | grep -v '^$$' | grep -v '^lib/'); do \
			base=$${line%.silica}; \
			if [ -x "$(THIS_DIR)/$$base" ]; then \
				{ cd "$(THIS_DIR)" && ./"$$base" 2>&1; echo $$?; } > "$(THIS_DIR)/$$base.sout" & \
				run_pid=$$!; elapsed=0; \
				while kill -0 "$$run_pid" 2>/dev/null && [ "$$elapsed" -lt 5 ]; do \
					sleep 1; elapsed=$$((elapsed + 1)); \
				done; \
				if kill -0 "$$run_pid" 2>/dev/null; then \
					kill -9 "$$run_pid" 2>/dev/null || true; \
					wait "$$run_pid" 2>/dev/null || true; \
					echo "TIMEOUT" > "$(THIS_DIR)/$$base.sout"; \
				else \
					wait "$$run_pid" 2>/dev/null || true; \
				fi; \
				if [ -f "$(THIS_DIR)/$$base.scout" ]; then \
					if ! diff -Bw -q "$(THIS_DIR)/$$base.sout" "$(THIS_DIR)/$$base.scout" > /dev/null 2>&1; then \
						echo "❌❌ $(MSG_PREFIX)$$base .sout differs from .scout"; \
						diff -Bw "$(THIS_DIR)/$$base.sout" "$(THIS_DIR)/$$base.scout" || true; \
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
	done; \
	[ "$$failed" -ne 0 ] && [ "$$ko" -eq 0 ] && ko=$$((ko + 1)); \
	printf '%d %d\n' "$$ok" "$$ko" > "$(THIS_DIR)/.integrate_counts"; \
	exit $$failed

clean:
	@cd "$(THIS_DIR)" && \
	rm -f *.sout *.sams *.o __silica_runtime.o __silica_runtime.sams \
		heap_binary_min_empty heap_binary_min_push_pop heap_binary_min_validate_invalid \
		heap_binary_min_priority_push_pop heap_binary_max_push_pop \
		silica.config silica.config.min silica.config.max .integrate_counts; \
	rm -f lib/*.sams lib/*.o
	@echo "✅ $(MSG_PREFIX)Clean complete"

help:
	@echo "heap_addition — RegionBinaryHeap trials (Phase 9)"
