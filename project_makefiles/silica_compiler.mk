# Shared silica-compiler invocation helpers for a project root Makefile.
# Include after setting THIS_DIR to the project root (trailing slash).
#
# Override on the command line if needed:
#   make SILICA_COMPILER=/path/to/silica-compiler

ifeq ($(filter command line,$(origin SILICA_COMPILER)),)
  ifeq ($(origin SILICA_COMPILER),undefined)
    SILICA_COMPILER := $(shell command -v silica-compiler 2>/dev/null)
  endif
endif

.PHONY: ensure-silica-compiler

ensure-silica-compiler:
	@if [ -z "$(SILICA_COMPILER)" ]; then \
		echo "FAIL: silica-compiler not found on PATH."; \
		echo "Install or build silica-compiler, or pass SILICA_COMPILER=/path/to/silica-compiler"; \
		exit 1; \
	fi; \
	if [ ! -x "$(SILICA_COMPILER)" ]; then \
		echo "FAIL: SILICA_COMPILER is not executable: $(SILICA_COMPILER)"; \
		exit 1; \
	fi

# Multi-unit batches use process-per-unit hygiene: the compiler exits 75 (EX_TEMPFAIL)
# between units so the OS reclaims host heap. Recipes must re-invoke until exit 0
# (or a hard failure). Usage inside a recipe (same cwd as silica.config):
#   $(RUN_SILICA_COMPILER)
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

define RUN_SILICA_COMPILER
	$(call RUN_SILICA_COMPILER_WITH,"$(SILICA_COMPILER)")
endef
