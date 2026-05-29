# Phase 1 — NodeIdAdjacencyGraph generated modules (List-aligned bracket naming, graph_representation_design.md §2.11).
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

LIB_SRC := ../../src/standard_data_structures

.DEFAULT_GOAL := all
.PHONY: all clean assembly objects executables integrate help silica.config lib-sources

lib-sources:
	@mkdir -p "$(THIS_DIR)lib"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_adj_list_helpers.silica)" "$(THIS_DIR)lib/graph_adj_list_helpers.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_adj_directed_mem_normal.silica)" "$(THIS_DIR)lib/graph_adj_directed_mem_normal.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_adj_undirected_mem_normal.silica)" "$(THIS_DIR)lib/graph_adj_undirected_mem_normal.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_adj_directed_int64_mem_normal.silica)" "$(THIS_DIR)lib/graph_adj_directed_int64_mem_normal.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_csr_directed_mem_normal.silica)" "$(THIS_DIR)lib/graph_csr_directed_mem_normal.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_csr_directed_int64_mem_normal.silica)" "$(THIS_DIR)lib/graph_csr_directed_int64_mem_normal.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_dense_directed_mem_normal.silica)" "$(THIS_DIR)lib/graph_dense_directed_mem_normal.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_dense_directed_int64_mem_normal.silica)" "$(THIS_DIR)lib/graph_dense_directed_int64_mem_normal.silica"

silica.config: lib-sources
	@cd "$(THIS_DIR)" && { \
		echo 'lib/graph_adj_list_helpers.silica'; \
		echo 'lib/graph_adj_directed_mem_normal.silica'; \
		echo 'lib/graph_adj_undirected_mem_normal.silica'; \
		echo 'lib/graph_adj_directed_int64_mem_normal.silica'; \
		echo 'lib/graph_csr_directed_mem_normal.silica'; \
		echo 'lib/graph_csr_directed_int64_mem_normal.silica'; \
		echo 'lib/graph_dense_directed_mem_normal.silica'; \
		echo 'lib/graph_dense_directed_int64_mem_normal.silica'; \
		echo 'graph_adj_directed_mem_normal_trial.silica'; \
		echo 'graph_adj_undirected_mem_normal_trial.silica'; \
		echo 'graph_adj_directed_int64_mem_normal_trial.silica'; \
		echo 'graph_csr_directed_mem_normal_trial.silica'; \
		echo 'graph_csr_directed_int64_mem_normal_trial.silica'; \
		echo 'graph_dense_directed_mem_normal_trial.silica'; \
		echo 'graph_dense_directed_int64_mem_normal_trial.silica'; \
		echo 'graph_reachability_csr_directed_mem_normal_trial.silica'; \
		echo 'graph_reachability_adj_directed_mem_normal_trial.silica'; \
		echo 'graph_degree_summary_csr_directed_mem_normal_trial.silica'; \
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
		lib_objs=""; \
		for lib_obj in lib/*.o; do \
			[ -f "$$lib_obj" ] && lib_objs="$$lib_objs $$lib_obj"; \
		done; \
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
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^lib/'); do \
		base=$${line%.silica}; \
		sams="$$base.sams"; \
		[ -f "$$sams" ] || continue; \
		if [ ! -f "$$base.ascomp" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$base has no .ascomp file"; \
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
		lib_objs=""; \
		for lib_obj in lib/*.o; do \
			[ -f "$$lib_obj" ] && lib_objs="$$lib_objs $$lib_obj"; \
		done; \
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
			{ ./"$$base" 2>&1; echo $$?; } > "$$base.sout"; \
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
	rm -f *.sout lib/*.sams lib/*.o lib/graph_adj_*.o lib/graph_csr_*.o lib/graph_dense_*.o *.sams __silica_runtime.o \
		graph_adj_directed_mem_normal_trial graph_adj_undirected_mem_normal_trial graph_adj_directed_int64_mem_normal_trial \
		graph_csr_directed_mem_normal_trial graph_csr_directed_int64_mem_normal_trial \
		graph_dense_directed_mem_normal_trial graph_dense_directed_int64_mem_normal_trial \
		graph_reachability_csr_directed_mem_normal_trial graph_reachability_adj_directed_mem_normal_trial \
		graph_degree_summary_csr_directed_mem_normal_trial \
		graph_adj_directed_mem_normal_trial.o graph_adj_undirected_mem_normal_trial.o graph_adj_directed_int64_mem_normal_trial.o \
		graph_csr_directed_mem_normal_trial.o graph_csr_directed_int64_mem_normal_trial.o \
		graph_dense_directed_mem_normal_trial.o graph_dense_directed_int64_mem_normal_trial.o \
		graph_reachability_csr_directed_mem_normal_trial.o graph_reachability_adj_directed_mem_normal_trial.o \
		graph_degree_summary_csr_directed_mem_normal_trial.o silica.config .integrate_counts
	@echo "✅ $(MSG_PREFIX)Clean complete"

help:
	@echo "graph_addition — Graph generated-structure trials (bracket naming per graph_representation_design.md §2.11)"
