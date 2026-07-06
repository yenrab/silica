# stdlib/data_structures — compiler-private representation cores (Layer 2+).
# Batch-build all modules listed in silica.config.

SILICA_COMPILER_SRC := ../../src
SILICA_COMPILER := $(SILICA_COMPILER_SRC)/silica-compiler

WBT_CORE_FILES := \
	wbt_set.silica \
	wbt_map.silica

.PHONY: all clean silica.config

ifdef QUIET
  ECHO := @true
else
  ECHO := @echo
endif

silica.config:
	@printf '%s\n' $(WBT_CORE_FILES) > silica.config

all: silica.config
	@if [ ! -x "$(SILICA_COMPILER)" ]; then \
		echo "FAIL: build silica-compiler: make -C $(SILICA_COMPILER_SRC)"; \
		exit 1; \
	fi
	@$(ECHO) "Compiling stdlib/data_structures..."
	@cd "$(dir $(abspath $(lastword $(MAKEFILE_LIST))))" && "$(SILICA_COMPILER)"
	@touch "$(dir $(abspath $(lastword $(MAKEFILE_LIST))))/.stdlib-built"

clean:
	@cd "$(dir $(abspath $(lastword $(MAKEFILE_LIST))))" && rm -f *.sams lib/*.sams *.o lib/*.o silica.config .stdlib-built

.DEFAULT_GOAL := all
