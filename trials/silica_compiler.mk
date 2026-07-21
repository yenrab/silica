# Shared seed host for trials — always binaries/silica-compiler unless overridden
# on the make command line (e.g. `make SILICA_COMPILER=/path/to/other integrate`).
# Paths are anchored to this file (trials/silica_compiler.mk).
#
# Usage (from any trials Makefile):
#   THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
#   include $(THIS_DIR)../silica_compiler.mk          # depth-1 trial
#   include $(THIS_DIR)../../silica_compiler.mk        # depth-2 trial
#   include silica_compiler.mk                        # trials/Makefile
#
# IMPORTANT: capture THIS_DIR (or MAKEFILE_DIR / LEAF_DIR) *before* this include.
# After include, $(lastword $(MAKEFILE_LIST)) is this file (trials/), so recipes that
# still use MAKEFILE_LIST write silica.config in the wrong directory and integrate SKIPs.
#
# Leaf helpers that already include this file (do not re-point the compiler elsewhere):
#   standard_data_structures_phase1/leaf.mk
#   ffi_addition/common_app.mk

_SILICA_COMPILER_MK_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
BINARIES_DIR := $(abspath $(_SILICA_COMPILER_MK_DIR)../binaries)
_SILICA_COMPILER_SEED := $(BINARIES_DIR)/silica-compiler

# Force the seed binary for undefined / environment origins. Command-line
# SILICA_COMPILER=... still wins (GNU make: origin "command line").
ifeq ($(filter command line,$(origin SILICA_COMPILER)),)
SILICA_COMPILER := $(_SILICA_COMPILER_SEED)
endif

UPDATE_SILICA_COMPILER_LINK := $(BINARIES_DIR)/update_silica_compiler_link.bash

# If silica-compiler is missing, run update_silica_compiler_link.bash and re-check.
define ENSURE_SILICA_COMPILER
	if [ ! -x "$(SILICA_COMPILER)" ]; then \
		echo "Missing $(SILICA_COMPILER); running update_silica_compiler_link.bash..."; \
		"$(UPDATE_SILICA_COMPILER_LINK)" || exit 1; \
		if [ ! -x "$(SILICA_COMPILER)" ]; then \
			echo "Missing seed host after update: $(SILICA_COMPILER)"; \
			exit 1; \
		fi; \
	fi
endef

.PHONY: ensure-silica-compiler
ensure-silica-compiler:
	@$(ENSURE_SILICA_COMPILER)

# Merged into every including Makefile's integrate (and compile/assembly when present).
integrate: ensure-silica-compiler
compile: ensure-silica-compiler
assembly: ensure-silica-compiler

# So prerequisites like `integrate: $(SILICA_COMPILER)` create the binaries/ link if needed.
$(SILICA_COMPILER):
	@$(ENSURE_SILICA_COMPILER)
