# Shared emit-target selection for src_selfhost.
# Include after setting THIS_DIR to the src_selfhost root (trailing slash).
#
# TARGET selects which emitter/<TARGET>/ tree is baked into silica-compiler
# and is written to silica.target as the project's emit_target declaration.
# This is an emit / code-generation backend, not a cross-compile CLI flag
# inside an already-built binary (single emitter_core per binary today).

EMITTER_ROOT := $(THIS_DIR)emitter

# Discover allowable emit targets from emitter/*/ directory names only.
# (macOS / GNU Make wildcard can also match plain files like Makefile.)
# Names must be simple identifiers: letters, digits, underscore.
ALLOWED_TARGETS := $(shell find "$(EMITTER_ROOT)" -mindepth 1 -maxdepth 1 -type d -exec basename {} \; 2>/dev/null | grep -E '^[A-Za-z0-9_]+$$' | LC_ALL=C sort -u)

HOST_UNAME_S := $(shell uname -s 2>/dev/null)
HOST_UNAME_M := $(shell uname -m 2>/dev/null)

# Default TARGET to the current host platform when the user does not pass
# TARGET=... on the command line or via the environment. Candidates are
# tried in order against directories that actually exist under emitter/.
ifndef TARGET
  TARGET_CANDIDATES :=
  ifeq ($(HOST_UNAME_S),Darwin)
    ifeq ($(HOST_UNAME_M),arm64)
      TARGET_CANDIDATES := apple_silicon_mac
    endif
  endif
  ifeq ($(HOST_UNAME_S),Linux)
    ifneq ($(filter $(HOST_UNAME_M),aarch64 arm64),)
      TARGET_CANDIDATES := linux_aarch64 aarch64_debian
    endif
    ifeq ($(HOST_UNAME_M),x86_64)
      TARGET_CANDIDATES := linux_x86_64
    endif
  endif
  TARGET := $(firstword $(filter $(TARGET_CANDIDATES),$(ALLOWED_TARGETS)))
endif

# True when TARGET was left to host detection (not set by user/env before include).
TARGET_ORIGIN := $(if $(filter command line environment,$(origin TARGET)),user,host-default)

.PHONY: check-target

check-target:
	@if [ -z "$(TARGET)" ]; then \
		echo "FAIL: could not detect host emit target (uname -s='$(HOST_UNAME_S)' -m='$(HOST_UNAME_M)')." >&2; \
		echo "Pass an explicit emit target, e.g.: make TARGET=apple_silicon_mac" >&2; \
		echo "Allowable targets (emitter/*/ ):" >&2; \
		for t in $(ALLOWED_TARGETS); do echo "  - $$t" >&2; done; \
		if [ -z "$(ALLOWED_TARGETS)" ]; then echo "  (none found under emitter/)" >&2; fi; \
		exit 1; \
	fi
	@echo "$(TARGET)" | grep -Eq '^[A-Za-z0-9_]+$$' || { \
		echo "FAIL: invalid emit target name '$(TARGET)' (use letters, digits, underscore only)." >&2; \
		exit 1; \
	}
	@if [ ! -d "$(EMITTER_ROOT)/$(TARGET)" ]; then \
		echo "FAIL: unknown emit target '$(TARGET)' (no directory emitter/$(TARGET)/)." >&2; \
		echo "Allowable targets:" >&2; \
		for t in $(ALLOWED_TARGETS); do echo "  - $$t" >&2; done; \
		if [ -z "$(ALLOWED_TARGETS)" ]; then echo "  (none found under emitter/)" >&2; fi; \
		echo "Example: make TARGET=apple_silicon_mac" >&2; \
		exit 1; \
	fi
	@if [ ! -f "$(EMITTER_ROOT)/$(TARGET)/emitter_core.silica" ]; then \
		echo "FAIL: emitter/$(TARGET)/ is missing emitter_core.silica." >&2; \
		exit 1; \
	fi

# Project-level emit declaration (real file). Recipe may run when check-target
# is asked for as a prereq path elsewhere; only bump mtime when TARGET changes.
silica.target: check-target
	@tmp="$(THIS_DIR)silica.target.tmp"; \
	printf 'emit_target: %s\n' '$(TARGET)' > "$$tmp"; \
	if ! cmp -s "$$tmp" "$(THIS_DIR)silica.target" 2>/dev/null; then \
		mv "$$tmp" "$(THIS_DIR)silica.target"; \
		echo "Wrote silica.target (emit_target: $(TARGET); source: $(TARGET_ORIGIN))"; \
	else \
		rm -f "$$tmp"; \
	fi
