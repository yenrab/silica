# Makefile for standard_data_structures generated modules.
# Compiled with the self-hosted silica-compiler (silica.config batch mode), not silica-boot.
# Runtime validation lives in trials/*_addition/.

SILICA_COMPILER := ../silica-compiler

ifdef QUIET
  ECHO := @true
else
  ECHO := @echo
endif

# Dependency order: helpers and registry first, then graph families.
BASE_CONFIG_FILES := \
	graph_adj_list_helpers.silica \
	structure_registry.silica \
	inline_type_expansion.silica \
	graph_adj_directed.silica \
	graph_adj_undirected.silica \
	graph_csr_directed.silica \
	graph_dense_directed.silica

BTREE_SET_CONFIG_FILES := \
	btree_set_csr.silica \
	btree_set_nodeid.silica

BTREE_MAP_CONFIG_FILES := \
	btree_nodeid.silica

BTREE_CSR_MAP_CONFIG_FILES := \
	btree_csr_map.silica

HEAP_BINARY_MIN_CONFIG_FILES := \
	heap_binary_min.silica

HEAP_BINARY_MAX_CONFIG_FILES := \
	heap_binary_max.silica

HEAP_DARY_MIN_CONFIG_FILES := \
	heap_dary_min.silica

SILICA_CONFIG_FILES := \
	$(BASE_CONFIG_FILES) \
	$(BTREE_SET_CONFIG_FILES) \
	$(BTREE_MAP_CONFIG_FILES) \
	$(BTREE_CSR_MAP_CONFIG_FILES) \
	$(HEAP_BINARY_MIN_CONFIG_FILES) \
	$(HEAP_BINARY_MAX_CONFIG_FILES) \
	$(HEAP_DARY_MIN_CONFIG_FILES)

CONFIG_FILES := \
	silica.config.base \
	silica.config.btree_set \
	silica.config.btree_map \
	silica.config.btree_csr_map \
	silica.config.heap_binary_min \
	silica.config.heap_binary_max \
	silica.config.heap_dary_min

SAMS_FILES := $(SILICA_CONFIG_FILES:.silica=.sams)

.DEFAULT_GOAL := all
.PHONY: all clean help silica.config $(CONFIG_FILES)

all: $(SILICA_COMPILER) $(CONFIG_FILES)
	$(ECHO) "Compiling standard_data_structures with silica-compiler..."
	@cd "$(CURDIR)" && cp silica.config.base silica.config && "$(abspath $(SILICA_COMPILER))"
	@cd "$(CURDIR)" && cp silica.config.btree_set silica.config && "$(abspath $(SILICA_COMPILER))"
	@cd "$(CURDIR)" && cp silica.config.btree_map silica.config && "$(abspath $(SILICA_COMPILER))"
	@cd "$(CURDIR)" && cp silica.config.btree_csr_map silica.config && "$(abspath $(SILICA_COMPILER))"
	@cd "$(CURDIR)" && cp silica.config.heap_binary_min silica.config && "$(abspath $(SILICA_COMPILER))"
	@cd "$(CURDIR)" && cp silica.config.heap_binary_max silica.config && "$(abspath $(SILICA_COMPILER))"
	@cd "$(CURDIR)" && cp silica.config.heap_dary_min silica.config && "$(abspath $(SILICA_COMPILER))"
	@echo "✅ standard_data_structures compiled with silica-compiler"

$(SILICA_COMPILER):
	@echo "Error: $(SILICA_COMPILER) not found. Run 'make -C .. executables' (or 'make -C .. build') first."
	@exit 1

silica.config: silica.config.base
	@cp silica.config.base silica.config

silica.config.base:
	@printf '%s\n' $(BASE_CONFIG_FILES) > $@

silica.config.btree_set:
	@printf '%s\n' $(BTREE_SET_CONFIG_FILES) > $@

silica.config.btree_map:
	@printf '%s\n' $(BTREE_MAP_CONFIG_FILES) > $@

silica.config.btree_csr_map:
	@printf '%s\n' $(BTREE_CSR_MAP_CONFIG_FILES) > $@

silica.config.heap_binary_min:
	@printf '%s\n' $(HEAP_BINARY_MIN_CONFIG_FILES) > $@

silica.config.heap_binary_max:
	@printf '%s\n' $(HEAP_BINARY_MAX_CONFIG_FILES) > $@

silica.config.heap_dary_min:
	@printf '%s\n' $(HEAP_DARY_MIN_CONFIG_FILES) > $@

silica.config.all:
	@printf '%s\n' $(SILICA_CONFIG_FILES) > $@

clean:
	@rm -f silica.config silica.config.all $(CONFIG_FILES) $(SAMS_FILES) __silica_runtime.sams
	@rm -f *.ll *.bc *.o
	@echo "✅ standard_data_structures clean complete"

help:
	@echo "standard_data_structures — compile via silica-compiler + silica.config"
	@echo "  Requires ../silica-compiler (build parent src/ first)"
	@echo "  Compiles in batches because generated families may share operation names."
	@echo "  Trials: ../trials/graph_addition/, ../trials/btree_*_addition/, ../trials/heap_addition/"
