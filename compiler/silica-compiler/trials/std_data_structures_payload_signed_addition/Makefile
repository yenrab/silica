# Phase 10.1 — signed integer payload cross-structure trials.
THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER := $(THIS_DIR)../../src/silica-compiler
TRIAL_DIR := $(notdir $(patsubst %/,%,$(THIS_DIR)))
MSG_PREFIX := $(TRIAL_DIR)/

RUST_SYSROOT := $(shell rustc --print sysroot 2>/dev/null)
RUST_TARGET := $(shell rustc -vV 2>/dev/null | sed -n "s/^host: *//p")
RUST_LLD := $(RUST_SYSROOT)/lib/rustlib/$(RUST_TARGET)/bin/rust-lld
ASSEMBLER := clang

MACOS_SDK := $(shell xcrun --sdk macosx --show-sdk-path 2>/dev/null)
MACOS_MIN_VERSION ?= 26.0
ASFLAGS_macos := -mmacosx-version-min=$(MACOS_MIN_VERSION)
LDFLAGS_rust-lld := -arch arm64 -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) -syslibroot $(MACOS_SDK) -lSystem -e main
LDFLAGS_clang := -Wl,-e,main -Wl,-macos_version_min,$(MACOS_MIN_VERSION)

GRAPH_TRIALS := signed_payload_graph_int8.silica signed_payload_graph_int16.silica signed_payload_graph_int32.silica signed_payload_graph_int64.silica
BTREE_TRIALS := signed_payload_btree_int8.silica signed_payload_btree_int16.silica signed_payload_btree_int32.silica signed_payload_btree_int64.silica
HEAP_TRIALS := signed_payload_heap_int8.silica signed_payload_heap_int16.silica signed_payload_heap_int32.silica signed_payload_heap_int64.silica
ALL_TRIALS := $(GRAPH_TRIALS) $(BTREE_TRIALS) $(HEAP_TRIALS)

LIB_SRC := ../../src/standard_data_structures

.DEFAULT_GOAL := all
.PHONY: all clean assembly objects executables integrate help silica.config lib-sources silica.config.graph silica.config.btree silica.config.heap

lib-sources:
	@mkdir -p "$(THIS_DIR)lib"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_adj_list_helpers.silica)" "$(THIS_DIR)lib/graph_adj_list_helpers.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_adj_directed.silica)" "$(THIS_DIR)lib/graph_adj_directed.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_nodeid.silica)" "$(THIS_DIR)lib/btree_nodeid.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/heap_binary_min.silica)" "$(THIS_DIR)lib/heap_binary_min.silica"

silica.config.graph: lib-sources
	@cd "$(THIS_DIR)" && { \
		echo 'lib/graph_adj_list_helpers.silica'; \
		echo 'lib/graph_adj_directed.silica'; \
		for src in $(GRAPH_TRIALS); do echo "$$src"; done; \
	} > silica.config.graph

silica.config.btree: lib-sources
	@cd "$(THIS_DIR)" && { \
		echo 'lib/btree_nodeid.silica'; \
		for src in $(BTREE_TRIALS); do echo "$$src"; done; \
	} > silica.config.btree

silica.config.heap: lib-sources
	@cd "$(THIS_DIR)" && { \
		echo 'lib/heap_binary_min.silica'; \
		for src in $(HEAP_TRIALS); do echo "$$src"; done; \
	} > silica.config.heap

silica.config: silica.config.graph silica.config.btree silica.config.heap
	@cd "$(THIS_DIR)" && cat silica.config.graph silica.config.btree silica.config.heap > silica.config

all: executables

assembly: silica.config.graph silica.config.btree silica.config.heap
	@cd "$(THIS_DIR)" && cp silica.config.graph silica.config && $(SILICA_COMPILER)
	@cd "$(THIS_DIR)" && cp silica.config.btree silica.config && $(SILICA_COMPILER)
	@cd "$(THIS_DIR)" && cp silica.config.heap silica.config && $(SILICA_COMPILER)
	@echo "✅ $(MSG_PREFIX)Assembly generated"

objects: assembly
	@cd "$(THIS_DIR)" && \
	for cfg in silica.config.graph silica.config.btree silica.config.heap; do \
		for line in $$(grep -v '^#' "$$cfg" 2>/dev/null | grep -v '^$$'); do \
			base=$${line%.silica}; sams="$$base.sams"; \
			[ -f "$$sams" ] || continue; \
			echo "Assembling: $$sams -> $$base.o"; \
			$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o" || exit 1; \
		done; \
	done; \
	if [ -f "__silica_runtime.sams" ]; then \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "__silica_runtime.sams" -o "__silica_runtime.o"; \
	fi

executables: objects
	@cd "$(THIS_DIR)" && \
	for src in $(GRAPH_TRIALS); do $(MAKE) --no-print-directory link-one BASE=$${src%.silica} LIB_OBJS="lib/graph_adj_directed.o lib/graph_adj_list_helpers.o"; done; \
	for src in $(BTREE_TRIALS); do $(MAKE) --no-print-directory link-one BASE=$${src%.silica} LIB_OBJS="lib/btree_nodeid.o"; done; \
	for src in $(HEAP_TRIALS); do $(MAKE) --no-print-directory link-one BASE=$${src%.silica} LIB_OBJS="lib/heap_binary_min.o"; done

link-one:
	@cd "$(THIS_DIR)" && \
	runtime_obj=""; [ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
	if test -x "$(RUST_LLD)"; then \
		$(RUST_LLD) -flavor darwin -o "$(BASE)" "$(BASE).o" $(LIB_OBJS) $$runtime_obj $(LDFLAGS_rust-lld) && echo "  ✅ $(MSG_PREFIX)$(BASE)"; \
	else \
		$(ASSEMBLER) "$(BASE).o" $(LIB_OBJS) $$runtime_obj -o "$(BASE)" $(LDFLAGS_clang) && echo "  ✅ $(MSG_PREFIX)$(BASE)"; \
	fi

integrate: silica.config.graph silica.config.btree silica.config.heap
	@rm -f .integrate_counts; ok=0; ko=0; failed=0; \
	rm -f *.sams lib/*.sams; \
	for cfg in graph btree heap; do \
		echo "Compiling signed payload $$cfg trials..."; \
		if ! (cd "$(THIS_DIR)" && cp silica.config.$$cfg silica.config && $(abspath $(SILICA_COMPILER))); then \
			echo "FAIL: signed payload $$cfg compilation failed"; \
			printf '%d %d\n' 0 1 > .integrate_counts; \
			exit 1; \
		fi; \
	done; \
	for cfg in silica.config.graph silica.config.btree silica.config.heap; do \
		for line in $$(grep -v '^#' "$$cfg" 2>/dev/null | grep -v '^$$'); do \
			base=$${line%.silica}; sams="$$base.sams"; \
			[ -f "$$sams" ] || continue; \
			echo "Assembling $$sams..."; \
			$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o" || { failed=1; break; }; \
		done; \
	done; \
	if [ -f "__silica_runtime.sams" ]; then \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "__silica_runtime.sams" -o "__silica_runtime.o" || failed=1; \
	fi; \
	for src in $(GRAPH_TRIALS); do base=$${src%.silica}; runtime_obj=""; [ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; echo "Linking $$base..."; $(RUST_LLD) -flavor darwin -o "$$base" "$$base.o" lib/graph_adj_directed.o lib/graph_adj_list_helpers.o $$runtime_obj $(LDFLAGS_rust-lld) || failed=1; done; \
	for src in $(BTREE_TRIALS); do base=$${src%.silica}; runtime_obj=""; [ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; echo "Linking $$base..."; $(RUST_LLD) -flavor darwin -o "$$base" "$$base.o" lib/btree_nodeid.o $$runtime_obj $(LDFLAGS_rust-lld) || failed=1; done; \
	for src in $(HEAP_TRIALS); do base=$${src%.silica}; runtime_obj=""; [ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; echo "Linking $$base..."; $(RUST_LLD) -flavor darwin -o "$$base" "$$base.o" lib/heap_binary_min.o $$runtime_obj $(LDFLAGS_rust-lld) || failed=1; done; \
	for src in $(ALL_TRIALS); do \
		base=$${src%.silica}; \
		if [ -x "$$base" ]; then \
			{ ./"$$base" 2>&1; echo $$?; } > "$$base.sout" & \
			run_pid=$$!; elapsed=0; \
			while kill -0 "$$run_pid" 2>/dev/null && [ "$$elapsed" -lt 5 ]; do sleep 1; elapsed=$$((elapsed + 1)); done; \
			if kill -0 "$$run_pid" 2>/dev/null; then kill -9 "$$run_pid" 2>/dev/null || true; wait "$$run_pid" 2>/dev/null || true; echo "TIMEOUT" > "$$base.sout"; else wait "$$run_pid" 2>/dev/null || true; fi; \
			if ! diff -Bw -q "$$base.sout" "$$base.scout" > /dev/null 2>&1; then \
				echo "❌❌ $(MSG_PREFIX)$$base .sout differs from .scout"; diff -Bw "$$base.sout" "$$base.scout" || true; ko=$$((ko + 1)); failed=1; \
			else \
				echo "✅✅ $(MSG_PREFIX)$$base output matches .scout"; ok=$$((ok + 1)); \
			fi; \
		fi; \
	done; \
	[ "$$failed" -ne 0 ] && [ "$$ko" -eq 0 ] && ko=$$((ko + 1)); \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	exit $$failed

clean:
	@cd "$(THIS_DIR)" && \
	rm -f *.sout *.sams *.o __silica_runtime.o __silica_runtime.sams \
		signed_payload_graph_int8 signed_payload_graph_int16 signed_payload_graph_int32 signed_payload_graph_int64 \
		signed_payload_btree_int8 signed_payload_btree_int16 signed_payload_btree_int32 signed_payload_btree_int64 \
		signed_payload_heap_int8 signed_payload_heap_int16 signed_payload_heap_int32 signed_payload_heap_int64 \
		silica.config silica.config.graph silica.config.btree silica.config.heap .integrate_counts; \
	rm -f lib/*.sams lib/*.o
	@echo "✅ $(MSG_PREFIX)Clean complete"

help:
	@echo "payload_signed_addition — Phase 10.1 signed integer payload trials"
