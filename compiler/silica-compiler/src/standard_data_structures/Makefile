# Makefile for standard_data_structures (Phase 0 registry + graph generated modules).
# Compiled with the self-hosted silica-compiler (silica.config batch mode), not silica-boot.
# Runtime validation lives in trials/graph_addition/.

SILICA_COMPILER := ../silica-compiler

ifdef QUIET
  ECHO := @true
else
  ECHO := @echo
endif

# Dependency order: helpers and registry first, then graph families.
SILICA_CONFIG_FILES := \
	graph_adj_list_helpers.silica \
	structure_registry.silica \
	inline_type_expansion.silica \
	graph_adj_directed_unweighted_normal.silica \
	graph_adj_undirected_unweighted_normal.silica \
	graph_adj_directed_weighted_int64_normal.silica \
	graph_csr_directed_unweighted_normal.silica \
	graph_csr_directed_weighted_int64_normal.silica \
	graph_dense_directed_unweighted_normal.silica \
	graph_dense_directed_weighted_int64_normal.silica

SAMS_FILES := $(SILICA_CONFIG_FILES:.silica=.sams)

.DEFAULT_GOAL := all
.PHONY: all clean help silica.config

all: $(SILICA_COMPILER) silica.config
	$(ECHO) "Compiling standard_data_structures with silica-compiler..."
	@cd "$(CURDIR)" && "$(abspath $(SILICA_COMPILER))"
	@echo "✅ standard_data_structures compiled with silica-compiler"

$(SILICA_COMPILER):
	@echo "Error: $(SILICA_COMPILER) not found. Run 'make -C .. executables' (or 'make -C .. build') first."
	@exit 1

silica.config:
	@printf '%s\n' $(SILICA_CONFIG_FILES) > $@

clean:
	@rm -f silica.config $(SAMS_FILES) __silica_runtime.sams
	@rm -f *.ll *.bc *.o
	@echo "✅ standard_data_structures clean complete"

help:
	@echo "standard_data_structures — compile via silica-compiler + silica.config"
	@echo "  Requires ../silica-compiler (build parent src/ first)"
	@echo "  Trials: ../trials/graph_addition/ (make integrate)"
