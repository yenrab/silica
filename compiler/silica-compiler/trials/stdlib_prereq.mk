# Shared prerequisite: build stdlib/data_structures when not yet compiled.
# Include from trial Makefiles after THIS_DIR and LIB_SRC are set.
#
#   STDLIB_PREREQ_MK := $(firstword $(wildcard $(THIS_DIR)../stdlib_prereq.mk $(THIS_DIR)../../stdlib_prereq.mk $(THIS_DIR)../../../stdlib_prereq.mk))
#   include $(STDLIB_PREREQ_MK)

ifndef THIS_DIR
$(error stdlib_prereq.mk requires THIS_DIR)
endif
ifndef LIB_SRC
$(error stdlib_prereq.mk requires LIB_SRC)
endif

STDLIB_DATA_STRUCTURES_DIR := $(abspath $(THIS_DIR)$(LIB_SRC))
STDLIB_BUILT_STAMP := $(STDLIB_DATA_STRUCTURES_DIR)/.stdlib-built

.PHONY: stdlib-data-structures

stdlib-data-structures: $(STDLIB_BUILT_STAMP)

$(STDLIB_BUILT_STAMP):
	@echo "Building stdlib/data_structures..."
	@$(MAKE) -C $(STDLIB_DATA_STRUCTURES_DIR) QUIET=1

include $(dir $(STDLIB_PREREQ_MK))link_lib_deps.mk
