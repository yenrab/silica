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

# Merged into every including Makefile's clean: drop published unit iface caches under
# that trial's CURDIR (make -C <trial> clean). Nested *.iface are included.
.PHONY: clean-silica-ifaces
clean: clean-silica-ifaces
clean-silica-ifaces:
	@find "$(CURDIR)" -name '*.iface' -type f -delete 2>/dev/null || true

# So prerequisites like `integrate: $(SILICA_COMPILER)` create the binaries/ link if needed.
$(SILICA_COMPILER):
	@$(ENSURE_SILICA_COMPILER)

# Multi-unit batches use process-per-unit hygiene: the seed exits 75 (EX_TEMPFAIL)
# between units so the OS reclaims host heap. Recipes must re-invoke until exit 0
# (or a hard failure). Usage inside a recipe (same cwd as silica.config):
#   $(RUN_SILICA_COMPILER)
# Optional override of the binary path:
#   $(call RUN_SILICA_COMPILER_WITH,$(abspath $(SILICA_COMPILER)))
# Quiet variant (no reclaim chatter) for golden-output capture:
#   $(call RUN_SILICA_COMPILER_QUIET_WITH,"$(SILICA_COMPILER)")
define RUN_SILICA_COMPILER_WITH
	while true; do \
		$(1); \
		ec=$$?; \
		if [ $$ec -eq 0 ]; then break; fi; \
		if [ $$ec -eq 75 ]; then \
			echo "  (reclaiming memory; continuing next unit)"; \
			continue; \
		fi; \
		exit $$ec; \
	done
endef

# Quiet reclaim loop for golden capture. Do not `exit` on hard failure: recipe lines that
# redirect into `.cur_fail` often run in the make shell (not a subshell), and `exit`
# would abort the recipe before the diff step. Callers use `|| true` around the loop.
define RUN_SILICA_COMPILER_QUIET_WITH
	while true; do \
		$(1); \
		ec=$$?; \
		if [ $$ec -eq 0 ]; then break; fi; \
		if [ $$ec -eq 75 ]; then continue; fi; \
		break; \
	done
endef

define RUN_SILICA_COMPILER
	$(call RUN_SILICA_COMPILER_WITH,"$(SILICA_COMPILER)")
endef

define RUN_SILICA_COMPILER_QUIET
	$(call RUN_SILICA_COMPILER_QUIET_WITH,"$(SILICA_COMPILER)")
endef
