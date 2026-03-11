# Makefile for Silica error enforcement trials
# Each .silica file is expected to produce a purposeful compiler failure.
# Compiles each file separately (rewrites silica.config per file), captures
# compiler output to .cur_fail, and diffs against .golden_fail.
# Reports ✅✅ when match, ❌❌ when differ. Exits with failure on any mismatch.
#
# Run from this directory: cd error_enforcement_addition && make

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER := $(THIS_DIR)../../src/silica-compiler

# All .silica files in this directory
SILICA_FILES := $(wildcard *.silica)

.DEFAULT_GOAL := all
.PHONY: all clean help

all:
	@failed=0; \
	cd "$(THIS_DIR)" && \
	for silica in $(SILICA_FILES); do \
		[ -f "$$silica" ] || continue; \
		base=$${silica%.silica}; \
		if [ ! -f "$$base.golden_fail" ]; then \
			echo "❌❌ $$base has no .golden_fail file"; \
			failed=1; \
			continue; \
		fi; \
		echo "$$silica" > silica.config; \
		$(SILICA_COMPILER) > "$$base.cur_fail" 2>&1 || true; \
		if diff -q "$$base.cur_fail" "$$base.golden_fail" > /dev/null 2>&1; then \
			echo "✅✅ $$base compiler failure matches .golden_fail"; \
		else \
			echo "❌❌ $$base .cur_fail differs from .golden_fail"; \
			diff "$$base.cur_fail" "$$base.golden_fail" || true; \
			failed=1; \
		fi; \
	done; \
	exit $$failed

clean:
	@cd "$(THIS_DIR)" && rm -f *.cur_fail
	@echo "✅ Clean complete"

help:
	@echo "Makefile - Silica error enforcement trials"
	@echo "=========================================="
	@echo "Each .silica file is compiled separately (silica.config rewritten per file)."
	@echo "Compiler output (expected failure) is captured to .cur_fail and compared to .golden_fail."
	@echo ""
	@echo "Targets:"
	@echo "  all   - Compile each .silica, diff .cur_fail vs .golden_fail (✅✅ match, ❌❌ differ)"
	@echo "  clean - Remove .cur_fail files"
	@echo "  help  - Show this help"
	@echo ""
	@echo "Silica files: $(SILICA_FILES)"
