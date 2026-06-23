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

GRAPH_TRIALS := $(sort $(notdir $(wildcard $(THIS_DIR)signed_payload_graph_*.silica)))
BTREE_TRIALS := $(sort $(notdir $(wildcard $(THIS_DIR)signed_payload_btree_*.silica)))
HEAP_TRIALS := $(sort $(notdir $(wildcard $(THIS_DIR)signed_payload_heap_*.silica)))
ALL_TRIALS := $(GRAPH_TRIALS) $(BTREE_TRIALS) $(HEAP_TRIALS)
TRIAL_BINARIES := $(ALL_TRIALS:.silica=)

LIB_SRC := ../../stdlib/data_structures

STDLIB_PREREQ_MK := $(firstword $(wildcard $(THIS_DIR)../stdlib_prereq.mk $(THIS_DIR)../../stdlib_prereq.mk $(THIS_DIR)../../../stdlib_prereq.mk))
include $(STDLIB_PREREQ_MK)

.DEFAULT_GOAL := all
.PHONY: all clean assembly objects executables integrate help silica.config lib-sources stdlib-data-structures

lib-sources: stdlib-data-structures
	@mkdir -p "$(THIS_DIR)lib"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_adj_list_helpers.silica)" "$(THIS_DIR)lib/graph_adj_list_helpers.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_adj_directed.silica)" "$(THIS_DIR)lib/graph_adj_directed.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_csr_map.silica)" "$(THIS_DIR)lib/btree_csr_map.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_nodeid.silica)" "$(THIS_DIR)lib/btree_nodeid.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/heap_binary_min.silica)" "$(THIS_DIR)lib/heap_binary_min.silica"

silica.config: lib-sources
	@cd "$(THIS_DIR)" && { \
		echo 'lib/graph_adj_list_helpers.silica'; \
		echo 'lib/graph_adj_directed.silica'; \
		echo 'lib/btree_csr_map.silica'; \
		echo 'lib/btree_nodeid.silica'; \
		echo 'lib/heap_binary_min.silica'; \
		for src in $(ALL_TRIALS); do echo "$$src"; done; \
	} > silica.config

all: executables

assembly: lib-sources
	@cd "$(THIS_DIR)" && { \
		echo 'lib/graph_adj_list_helpers.silica'; \
		echo 'lib/graph_adj_directed.silica'; \
		for src in $(GRAPH_TRIALS); do echo "$$src"; done; \
	} > silica.config && $(SILICA_COMPILER)
	@cd "$(THIS_DIR)" && { \
		echo 'lib/btree_csr_map.silica'; \
		echo 'lib/btree_nodeid.silica'; \
		for src in $(BTREE_TRIALS); do echo "$$src"; done; \
	} > silica.config && $(SILICA_COMPILER)
	@cd "$(THIS_DIR)" && { \
		echo 'lib/heap_binary_min.silica'; \
		for src in $(HEAP_TRIALS); do echo "$$src"; done; \
	} > silica.config && $(SILICA_COMPILER)
	@echo "✅ $(MSG_PREFIX)Assembly generated"

objects: assembly
	@cd "$(THIS_DIR)" && \
	for src in $(ALL_TRIALS); do \
		base=$${src%.silica}; sams="$$base.sams"; \
		[ -f "$$sams" ] || continue; \
		echo "Assembling: $$sams -> $$base.o"; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o" || exit 1; \
	done; \
	for lib_sams in lib/graph_adj_list_helpers.sams lib/graph_adj_directed.sams lib/btree_csr_map.sams lib/btree_nodeid.sams lib/heap_binary_min.sams; do \
		[ -f "$$lib_sams" ] || continue; \
		base=$${lib_sams%.sams}; \
		echo "Assembling: $$lib_sams -> $$base.o"; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$lib_sams" -o "$$base.o" || exit 1; \
	done; \
	if [ -f "__silica_runtime.sams" ]; then \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "__silica_runtime.sams" -o "__silica_runtime.o"; \
	fi

executables: objects
	@cd "$(THIS_DIR)" && \
	for src in $(ALL_TRIALS); do \
		base=$${src%.silica}; \
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

integrate: lib-sources
	@cd "$(THIS_DIR)" && \
	rm -f .integrate_counts; ok=0; ko=0; failed=0; \
	rm -f *.sams lib/*.sams $(TRIAL_BINARIES) silica.config; \
	{ \
		echo 'lib/graph_adj_list_helpers.silica'; \
		echo 'lib/graph_adj_directed.silica'; \
		for src in $(GRAPH_TRIALS); do echo "$$src"; done; \
	} > silica.config; \
	echo "Compiling signed payload graph trials..."; \
	if ! $(abspath $(SILICA_COMPILER)); then \
		echo "FAIL: signed payload graph compilation failed"; \
		printf '%d %d\n' 0 1 > .integrate_counts; exit 1; \
	fi; \
	{ \
		echo 'lib/btree_csr_map.silica'; \
		echo 'lib/btree_nodeid.silica'; \
		for src in $(BTREE_TRIALS); do echo "$$src"; done; \
	} > silica.config; \
	echo "Compiling signed payload btree trials..."; \
	if ! $(abspath $(SILICA_COMPILER)); then \
		echo "FAIL: signed payload btree compilation failed"; \
		printf '%d %d\n' 0 1 > .integrate_counts; exit 1; \
	fi; \
	{ \
		echo 'lib/heap_binary_min.silica'; \
		for src in $(HEAP_TRIALS); do echo "$$src"; done; \
	} > silica.config; \
	echo "Compiling signed payload heap trials..."; \
	if ! $(abspath $(SILICA_COMPILER)); then \
		echo "FAIL: signed payload heap compilation failed"; \
		printf '%d %d\n' 0 1 > .integrate_counts; exit 1; \
	fi; \
	for src in $(ALL_TRIALS); do \
		base=$${src%.silica}; sams="$$base.sams"; \
		[ -f "$$sams" ] || continue; \
		echo "Assembling $$sams..."; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o" || { failed=1; break; }; \
	done; \
	for lib_sams in lib/graph_adj_list_helpers.sams lib/graph_adj_directed.sams lib/btree_csr_map.sams lib/btree_nodeid.sams lib/heap_binary_min.sams; do \
		[ -f "$$lib_sams" ] || continue; \
		base=$${lib_sams%.sams}; \
		echo "Assembling $$lib_sams..."; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$lib_sams" -o "$$base.o" || failed=1; \
	done; \
	if [ -f "__silica_runtime.sams" ]; then \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "__silica_runtime.sams" -o "__silica_runtime.o" || failed=1; \
	fi; \
	for src in $(ALL_TRIALS); do \
		base=$${src%.silica}; \
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
		$(TRIAL_BINARIES) \
		silica.config .integrate_counts; \
	rm -f lib/*.sams lib/*.o
	@echo "✅ $(MSG_PREFIX)Clean complete"

help:
	@echo "payload_signed_addition — Phase 10.1 signed integer payload trials"
	@echo "Trials: $(ALL_TRIALS)"
