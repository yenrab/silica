# Multi-file trial: library exports one function; main uses it then calls a non-exported sibling (ParseError E1011, spec §19.2).
# Run from this directory or via ../Makefile `all`.

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
# Parent passes COMPILER=path to silica-compiler
COMPILER ?= $(THIS_DIR)../../../src/silica-compiler

.PHONY: integrate check clean

integrate: check

check:
	@cd "$(THIS_DIR)" && $(COMPILER) > main_use_partial.cur_fail 2>&1 || true
	@if diff -q "$(THIS_DIR)main_use_partial.cur_fail" "$(THIS_DIR)main_use_partial.golden_fail" > /dev/null 2>&1; then \
		echo "✅✅ module_nonexported_callee_trial"; \
	else \
		echo "❌❌ module_nonexported_callee_trial: .cur_fail differs from .golden_fail"; \
		diff "$(THIS_DIR)main_use_partial.cur_fail" "$(THIS_DIR)main_use_partial.golden_fail" || true; \
		exit 1; \
	fi

clean:
	@rm -f "$(THIS_DIR)main_use_partial.cur_fail"
