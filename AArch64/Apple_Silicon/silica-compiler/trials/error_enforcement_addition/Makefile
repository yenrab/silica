# Makefile for Silica error enforcement trials
# Files to test are read from silica.config (never modified by this Makefile).
# Each file is compiled in isolation inside a temporary directory: a symlink to
# the .silica file and a single-entry silica.config are created there, the
# compiler is run, then the temp directory is removed.
# Compiler output is captured to *.cur_fail and diffed against *.golden_fail.
# Reports ✅✅ on match, ❌❌ on mismatch or missing .golden_fail.
# Exits with failure if any check fails.
#
# Run from this directory: cd error_enforcement_addition && make

THIS_DIR        := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER := $(THIS_DIR)../../src/silica-compiler

# Authoritative file list — read once at parse time; silica.config is never written.
SILICA_FILES := $(shell cat $(THIS_DIR)silica.config)

.DEFAULT_GOAL := all
.PHONY: all integrate clean help

# Same checks as top-level trials Makefile for this directory (error output vs .golden_fail).
integrate: all

all:
	@failed=0; \
	for silica in $(SILICA_FILES); do \
		[ -f "$(THIS_DIR)$$silica" ] || { echo "❌❌ $$silica: file not found"; failed=1; continue; }; \
		base=$${silica%.silica}; \
		if [ ! -f "$(THIS_DIR)$$base.golden_fail" ]; then \
			echo "❌❌ $$base: no matching .golden_fail file"; \
			failed=1; \
			continue; \
		fi; \
		tmpdir=$$(mktemp -d); \
		ln -s "$(THIS_DIR)$$silica" "$$tmpdir/$$silica"; \
		echo "$$silica" > "$$tmpdir/silica.config"; \
		(cd "$$tmpdir" && $(SILICA_COMPILER)) > "$(THIS_DIR)$$base.cur_fail" 2>&1 || true; \
		rm -rf "$$tmpdir"; \
		if diff -q "$(THIS_DIR)$$base.cur_fail" "$(THIS_DIR)$$base.golden_fail" > /dev/null 2>&1; then \
			echo "✅✅ $$base"; \
		else \
			echo "❌❌ $$base: .cur_fail differs from .golden_fail"; \
			diff "$(THIS_DIR)$$base.cur_fail" "$(THIS_DIR)$$base.golden_fail" || true; \
			failed=1; \
		fi; \
	done; \
	exit $$failed

clean:
	@rm -f $(THIS_DIR)*.cur_fail
	@echo "✅ Clean complete"

help:
	@echo "Makefile - Silica error enforcement trials"
	@echo "=========================================="
	@echo "File list is read from silica.config (never modified by this Makefile)."
	@echo "Each file is compiled in an isolated temp directory so silica.config"
	@echo "is left untouched.  Compiler output is captured to .cur_fail and"
	@echo "compared to .golden_fail."
	@echo ""
	@echo "Targets:"
	@echo "  all       - Compile each .silica, diff .cur_fail vs .golden_fail"
	@echo "              ✅✅ = output matches .golden_fail"
	@echo "              ❌❌ = output differs, file not found, or missing .golden_fail"
	@echo "  integrate - Same as all (integration test for this directory)"
	@echo "  clean     - Remove .cur_fail files"
	@echo "  help      - Show this help"
	@echo ""
	@echo "Files under test: $(SILICA_FILES)"
