# Phase 0.4 standard data structure trait-constructor-record trials.

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

LIB_SRC := ../../src/standard_data_structures

.DEFAULT_GOAL := all
.PHONY: all clean assembly objects executables integrate help silica.config lib-sources

lib-sources:
	@mkdir -p "$(THIS_DIR)lib"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/OrderedSet.silica)" "$(THIS_DIR)lib/OrderedSet.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/ordered_set_nodeid_adapter.silica)" "$(THIS_DIR)lib/ordered_set_nodeid_adapter.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/ordered_set_csr_adapter.silica)" "$(THIS_DIR)lib/ordered_set_csr_adapter.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/OrderedMap.silica)" "$(THIS_DIR)lib/OrderedMap.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/ordered_map_nodeid_adapter.silica)" "$(THIS_DIR)lib/ordered_map_nodeid_adapter.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/ordered_map_csr_adapter.silica)" "$(THIS_DIR)lib/ordered_map_csr_adapter.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/Heap.silica)" "$(THIS_DIR)lib/Heap.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/DirectedGraph.silica)" "$(THIS_DIR)lib/DirectedGraph.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_set_nodeid.silica)" "$(THIS_DIR)lib/btree_set_nodeid.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_set_csr.silica)" "$(THIS_DIR)lib/btree_set_csr.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_nodeid.silica)" "$(THIS_DIR)lib/btree_nodeid.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_csr_map.silica)" "$(THIS_DIR)lib/btree_csr_map.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_set_phase04.silica)" "$(THIS_DIR)lib/btree_set_phase04.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/heap_phase04.silica)" "$(THIS_DIR)lib/heap_phase04.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_phase04.silica)" "$(THIS_DIR)lib/graph_phase04.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_adj_list_helpers.silica)" "$(THIS_DIR)lib/graph_adj_list_helpers.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_adj_directed.silica)" "$(THIS_DIR)lib/graph_adj_directed.silica"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/graph_adj_undirected.silica)" "$(THIS_DIR)lib/graph_adj_undirected.silica"

silica.config: lib-sources
	@cd "$(THIS_DIR)" && { \
		echo 'lib/OrderedSet.silica'; \
		echo 'lib/ordered_set_nodeid_adapter.silica'; \
		echo 'lib/ordered_set_csr_adapter.silica'; \
		echo 'lib/OrderedMap.silica'; \
		echo 'lib/ordered_map_nodeid_adapter.silica'; \
		echo 'lib/ordered_map_csr_adapter.silica'; \
		echo 'lib/Heap.silica'; \
		echo 'lib/DirectedGraph.silica'; \
		echo 'lib/btree_set_csr.silica'; \
		echo 'lib/btree_set_nodeid.silica'; \
		echo 'lib/btree_nodeid.silica'; \
		echo 'lib/btree_csr_map.silica'; \
		echo 'lib/btree_set_phase04.silica'; \
		echo 'lib/heap_phase04.silica'; \
		echo 'lib/graph_phase04.silica'; \
		echo 'lib/graph_adj_list_helpers.silica'; \
		echo 'lib/graph_adj_directed.silica'; \
		echo 'lib/graph_adj_undirected.silica'; \
		echo 'ordered_set_constructor_record_trait.silica'; \
		echo 'heap_constructor_record_trait.silica'; \
		echo 'directed_graph_trait.silica'; \
		echo 'btree_set_nodeid_constructor_record.silica'; \
		echo 'btree_nodeid_map_trait_names.silica'; \
        echo 'ordered_map_btree_shape_trait.silica'; \
        echo 'ordered_set_witness_int64.silica'; \
        echo 'ordered_set_witness_string.silica'; \
        echo 'string_set_phase04.silica'; \
        echo 'trait_link_name_dual_impl.silica'; \
        echo 'heap_witness_int64.silica'; \
        echo 'ordered_set_nodeid_size_trait.silica'; \
        echo 'ordered_map_compare_value_trait.silica'; \
        echo 'ordered_set_csr_compare_item_trait.silica'; \
        echo 'directed_graph_adj_empty_trait.silica'; \
        echo 'directed_graph_adj_lookup.silica'; \
        echo 'directed_graph_adj_add_edge.silica'; \
        echo 'undirected_graph_adj_add_edge.silica'; \
        echo 'undirected_graph_adj_self_edge.silica'; \
        echo 'directed_graph_adj_weight_add.silica'; \
        echo 'directed_graph_adj_weight_missing.silica'; \
        echo 'directed_graph_adj_validate_pass.silica'; \
        echo 'directed_graph_adj_validate_bad_edge_count.silica'; \
        echo 'directed_graph_adj_validate_bad_neighbor.silica'; \
        echo 'undirected_graph_adj_validate_pass.silica'; \
        echo 'undirected_graph_adj_validate_mirror_fail.silica'; \
        echo 'directed_graph_witness_int64.silica'; \
    } > silica.config

all: executables

assembly: clean silica.config
	@echo "Compiling with silica-compiler..."
	@cd "$(THIS_DIR)" && $(SILICA_COMPILER)
	@echo "✅ $(MSG_PREFIX)Assembly generated"

objects: assembly
	@cd "$(THIS_DIR)" && \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$'); do \
		base=$${line%.silica}; \
		sams="$$base.sams"; \
		[ -f "$$sams" ] || continue; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o" || exit 1; \
	done; \
	if [ -f "__silica_runtime.sams" ]; then \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "__silica_runtime.sams" -o "__silica_runtime.o"; \
	fi

executables: objects
	@cd "$(THIS_DIR)" && \
	for line in ordered_set_constructor_record_trait.silica heap_constructor_record_trait.silica directed_graph_trait.silica btree_set_nodeid_constructor_record.silica btree_nodeid_map_trait_names.silica ordered_map_btree_shape_trait.silica ordered_set_nodeid_size_trait.silica ordered_map_compare_value_trait.silica ordered_set_csr_compare_item_trait.silica directed_graph_adj_empty_trait.silica directed_graph_adj_lookup.silica directed_graph_adj_add_edge.silica undirected_graph_adj_add_edge.silica undirected_graph_adj_self_edge.silica directed_graph_adj_weight_add.silica directed_graph_adj_weight_missing.silica directed_graph_adj_validate_pass.silica directed_graph_adj_validate_bad_edge_count.silica directed_graph_adj_validate_bad_neighbor.silica undirected_graph_adj_validate_pass.silica undirected_graph_adj_validate_mirror_fail.silica; do \
		base=$${line%.silica}; \
		lib_objs=""; \
		for lib_obj in lib/*.o; do \
			[ -f "$$lib_obj" ] && lib_objs="$$lib_objs $$lib_obj"; \
		done; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o "$$base" "$$base.o" $$lib_objs $$runtime_obj $(LDFLAGS_rust-lld) || exit 1; \
		else \
			$(ASSEMBLER) "$$base.o" $$lib_objs $$runtime_obj -o "$$base" $(LDFLAGS_clang) || exit 1; \
		fi; \
		echo "  ✅ $(MSG_PREFIX)$$base"; \
	done

integrate: clean silica.config
	@rm -f .integrate_counts; ok=0; ko=0; failed=0; \
	cd "$(THIS_DIR)" && \
	if ! $(SILICA_COMPILER); then \
		echo "FAIL: Compilation failed"; \
		printf '%d %d\n' 0 1 > .integrate_counts; \
		exit 1; \
	fi; \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$'); do \
		base=$${line%.silica}; \
		sams="$$base.sams"; \
		[ -f "$$sams" ] || continue; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o" || { failed=1; break; }; \
	done; \
	if [ -f "__silica_runtime.sams" ]; then \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "__silica_runtime.sams" -o "__silica_runtime.o" || failed=1; \
	fi; \
	for line in ordered_set_constructor_record_trait.silica heap_constructor_record_trait.silica directed_graph_trait.silica btree_set_nodeid_constructor_record.silica btree_nodeid_map_trait_names.silica ordered_map_btree_shape_trait.silica ordered_set_nodeid_size_trait.silica ordered_map_compare_value_trait.silica ordered_set_csr_compare_item_trait.silica directed_graph_adj_empty_trait.silica directed_graph_adj_lookup.silica directed_graph_adj_add_edge.silica undirected_graph_adj_add_edge.silica undirected_graph_adj_self_edge.silica directed_graph_adj_weight_add.silica directed_graph_adj_weight_missing.silica directed_graph_adj_validate_pass.silica directed_graph_adj_validate_bad_edge_count.silica directed_graph_adj_validate_bad_neighbor.silica undirected_graph_adj_validate_pass.silica undirected_graph_adj_validate_mirror_fail.silica; do \
		base=$${line%.silica}; \
		lib_objs=""; \
		for lib_obj in lib/*.o; do \
			[ -f "$$lib_obj" ] && lib_objs="$$lib_objs $$lib_obj"; \
		done; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o "$$base" "$$base.o" $$lib_objs $$runtime_obj $(LDFLAGS_rust-lld) || failed=1; \
		else \
			$(ASSEMBLER) "$$base.o" $$lib_objs $$runtime_obj -o "$$base" $(LDFLAGS_clang) || failed=1; \
		fi; \
	done; \
	for line in ordered_set_constructor_record_trait.silica heap_constructor_record_trait.silica directed_graph_trait.silica btree_set_nodeid_constructor_record.silica btree_nodeid_map_trait_names.silica ordered_map_btree_shape_trait.silica ordered_set_nodeid_size_trait.silica ordered_map_compare_value_trait.silica ordered_set_csr_compare_item_trait.silica directed_graph_adj_empty_trait.silica directed_graph_adj_lookup.silica directed_graph_adj_add_edge.silica undirected_graph_adj_add_edge.silica undirected_graph_adj_self_edge.silica directed_graph_adj_weight_add.silica directed_graph_adj_weight_missing.silica directed_graph_adj_validate_pass.silica directed_graph_adj_validate_bad_edge_count.silica directed_graph_adj_validate_bad_neighbor.silica undirected_graph_adj_validate_pass.silica undirected_graph_adj_validate_mirror_fail.silica; do \
		base=$${line%.silica}; \
		if [ -x "$$base" ]; then \
			{ ./"$$base" 2>&1; echo $$?; } > "$$base.sout"; \
			if [ -f "$$base.scout" ] && diff -Bw -q "$$base.sout" "$$base.scout" > /dev/null 2>&1; then \
				echo "✅✅ $(MSG_PREFIX)$$base output matches .scout"; ok=$$((ok + 1)); \
			else \
				echo "❌❌ $(MSG_PREFIX)$$base .sout differs from .scout"; \
				[ -f "$$base.scout" ] && diff -Bw "$$base.sout" "$$base.scout" || true; \
				ko=$$((ko + 1)); failed=1; \
			fi; \
		fi; \
	done; \
	for line in ordered_set_witness_int64.silica ordered_set_witness_string.silica heap_witness_int64.silica directed_graph_witness_int64.silica; do \
		base=$${line%.silica}; \
		if [ -f "$$base.sams" ]; then \
			echo "✅✅ $(MSG_PREFIX)$$base compiled (witness trial)"; ok=$$((ok + 1)); \
		else \
			echo "❌❌ $(MSG_PREFIX)$$base failed to compile"; ko=$$((ko + 1)); failed=1; \
		fi; \
	done; \
	for line in trait_link_name_dual_impl.silica; do \
		base=$${line%.silica}; \
		sams="$$base.sams"; \
		if [ ! -f "$$sams" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$base: missing $$sams"; ko=$$((ko + 1)); failed=1; \
			continue; \
		fi; \
		if [ ! -f "$$base.ascomp" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$base has no .ascomp file"; ko=$$((ko + 1)); failed=1; \
		elif ! diff -Bw -q "$$sams" "$$base.ascomp" > /dev/null 2>&1; then \
			echo "❌❌ $(MSG_PREFIX)$$base .sams differs from .ascomp"; \
			diff -Bw "$$sams" "$$base.ascomp" || true; \
			ko=$$((ko + 1)); failed=1; \
		else \
			echo "✅✅ $(MSG_PREFIX)$$base assembly matches .ascomp (link-name mangling)"; ok=$$((ok + 1)); \
		fi; \
	done; \
	[ "$$failed" -ne 0 ] && [ "$$ko" -eq 0 ] && ko=$$((ko + 1)); \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	exit $$failed

clean:
	@cd "$(THIS_DIR)" && rm -f *.sout lib/*.sams lib/*.o lib/*.silica *.sams *.o __silica_runtime.o ordered_set_constructor_record_trait heap_constructor_record_trait directed_graph_trait btree_set_nodeid_constructor_record btree_nodeid_map_trait_names ordered_map_btree_shape_trait ordered_set_nodeid_size_trait ordered_map_compare_value_trait ordered_set_csr_compare_item_trait directed_graph_adj_empty_trait directed_graph_adj_lookup directed_graph_adj_add_edge undirected_graph_adj_add_edge undirected_graph_adj_self_edge directed_graph_adj_weight_add directed_graph_adj_weight_missing directed_graph_adj_validate_pass directed_graph_adj_validate_bad_edge_count directed_graph_adj_validate_bad_neighbor undirected_graph_adj_validate_pass undirected_graph_adj_validate_mirror_fail silica.config .integrate_counts
	@echo "✅ $(MSG_PREFIX)Clean complete"

help:
	@echo "standard_data_structures_phase04_addition — constructor-record and standard trait dispatch trials"
